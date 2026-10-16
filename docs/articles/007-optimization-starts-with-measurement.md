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

Duplicate accounting is also bounded rather than tied to an ever-growing event set. The production
observer uses a fixed array of atomic version slots indexed by the immutable entry version. The
first in-flight delivery claims its slot; a repeated delivery of the same version becomes a no-op.
A collision with another in-flight version does not guess: it marks the observer epoch dirty so an
exact snapshot requires reconciliation. This trades an impossible promise of collision-free
deduplication for fixed memory and fail-closed semantics.

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

The first implementation used a bounded Tokio channel and immediately demonstrated why this stage
belongs on a local machine: although correctness passed, publication and drain allocated a median
78.47 bytes per removal. Replacing it with a preallocated bounded ring removed those allocations.
On the exact committed reference-model binary, both the atomic-counter control and the versioned
observer reported 0 gross allocated bytes per operation in all three counterbalanced repetitions.
The tiny-run elapsed values were retained but not promoted; the model does not yet contain Moka's
automatic-removal delivery. The result proves feasibility of the HydraCache-side lifecycle, not the
backend integration.

The first isolated Moka patch then separated key locking from removal delivery. It added a
lab-only observer builder mode that reused the existing delivery machinery but did not create the
listener key-lock map. Median insert allocation became identical to listener-off: 400.44 B/op for
both, versus 755.74 B/op with the ordinary listener. This directly confirmed that the key-lock path
owned the fill penalty rather than merely correlating with it.

Removal did not fall all the way to off. The observer measured 2,700.16 B/op, compared with
2,276.23 off and 3,044.19 with the listener. Removing key locks cut 44.8% of the listener's
incremental removal allocation, but the patch deliberately retained boxed listener futures and
cancellation guards. That residual became the next isolated factor. The same spike verified real
delivery for explicit removal, replacement, expiry, and capacity eviction, so the lower insert cost
was not obtained by silently dropping an automatic-removal class.

The second patch stopped representing the observer as an async listener at all. It called the
synchronous observer directly at replacement, invalidation, expiry, and capacity-removal sites;
ordinary listeners retained their existing locks, futures, cancellation safety, and behavior. The
observer then matched listener-off insert allocation exactly at 400.69 B/op. Its remove median was
2,275.41 B/op versus 2,287.78 off. We interpret the small negative difference as no detected
allocation penalty, not as a speedup from observing removals.

Finally, the harness connected those real Moka callbacks to the versioned bounded-cleanup model.
It inserted key version 41, replaced it with version 42, delayed cleanup of 41 until after 42 was
registered, and proved that version 42's membership and retained-byte accounting survived. At that
point the local spike had implemented and falsified the complete proposed lifecycle: automatic
delivery, immediate bounded accounting, duplicate suppression, conditional deferred cleanup,
saturation/dirty epochs, reconciliation, shutdown drain, and fail-closed exact snapshots.

At that point the result still did not authorize a product change. The successful code was an
isolated patch against a development copy of Moka, not HydraCache's locked production dependency.
The remaining boundary was governance and qualification: recorded review, baseline-only
allocation/RSS rejection limits, an explicit dependency decision, D2 authorization, and only then
product integration plus the full local and dedicated-host matrices.

We turned that boundary into data rather than leaving it as a sentence in a plan. The D2 review
candidate names the exact authorized surfaces, upstream-first and pinned-fork dependency choices,
rollback, correctness falsifiers, and D3 measurements. Its proposed 15% fill-allocation minimum is
derived from the smaller baseline-only listener overhead, not from the successful observer spike.
Candidate evidence is kept in a separate exclusion list. Allocation and RSS guards reuse the
previously reviewed practical envelopes. Because this is a single-maintainer project, we recorded a
proposal-scoped exception instead of pretending that automation was an independent reviewer. The
exception binds the earlier threshold commit by full SHA, separates governance, dependency,
implementation, and measurement commits, retains every attempt, preserves dedicated-host
qualification, and requires later publication to say “self-reviewed”. Completing a convincing
prototype still only prepares the decision; it does not grant the prototype permission to become
the product.

### Turning a successful patch into an owned dependency

We chose the pinned-fork path for the 0.73 integration window. That choice is narrower than “use
our branch”: HydraCache consumes only the exact crates.io package
`hydra-moka =0.12.15-hydra.1`, whose registry checksum is
`7ad8a0701236306b753373994b7077769ad5c2d6dbc60ae31c6258937ab6165a`. The package preserves the
Rust library name `moka`, while its provenance binds the runtime observer code to commit
`352e53faa480c9997272b9c70798dd5b5c15d581` in the project-owned fork. A branch name is useful for
humans but mutable, so the contract rejects it as a dependency identity. The fork is one focused
runtime commit over Moka `v0.12.15`; the receipt also records both source-tree ids,
a stable patch id, and the digest of the earlier checked-in prototype patch. These identities make
it possible to distinguish a reviewed source change from a later force-push or unrelated fork edit.

Owning the repository does not make the dependency trustworthy by itself. We ran Moka's complete
all-feature tests and doctests, denied every Clippy warning, verified the publishable package with
the `future` feature, checked the declared Rust 1.71.1 MSRV using upstream-compatible dependency
pins, and applied HydraCache's `cargo-deny` policy. We generated a CycloneDX 1.5 inventory and bound
its SHA-256 to the decision. A feature-powerset run checked all 80 valid combinations containing
`sync` or `future`, while target checks covered Windows x86_64 plus Linux x86_64 and aarch64. The
isolated HydraCache harness then re-proved every removal cause and the delayed old-version cleanup
case against the exact fork code.

The review also changed the prototype in a safety-relevant way before it was pinned. An observer
panic is caught; the observer is disabled after its first panic, and later cache operations remain
usable. The API documentation prohibits blocking, I/O, and reentering the same cache. This does not
prove that HydraCache's integration is correct—it makes the dependency contract precise enough for
product tests to try to falsify it.

We later opened upstream discussion `moka-rs/moka#606` and draft pull request `#607`, but their
review remains independent of the HydraCache release clock. The receipt says `pending`, rather than
implying that the Moka maintainers reviewed or accepted the API. An external review has an unbounded
schedule, while a fork we own has explicit maintenance and rollback costs. We accepted those costs:
recheck upstream and advisories at least monthly and before
each release candidate, never broaden or move the exact package version in place, and require a new
receipt, registry checksum, lockfile diff, SBOM, and full dependency gate for every version change.
Rollback is one product
commit restoring crates.io Moka 0.12.15 and the previous listener wiring.

D2 therefore opens exactly one door: a later product-integration commit may use the pinned observer
seam on the pre-authorized files and surfaces. It does not open measurement. Candidate runs remain
forbidden until explicit removal, replacement, expiry, capacity eviction, duplicate delivery,
saturation, reconciliation, cancellation, shutdown, reentrancy, panic, compatibility, and rollback
tests pass in HydraCache. Local evidence will still be non-promotable, and the numerical claim still
requires the admitted dedicated-host pairs frozen earlier.

### What changed when the observer entered the product

The product integration landed as commit
`73fc38a131d26e78b246fe93d5edd71d33796bbf`. It originally pinned the reviewed Moka revision in the
manifest and lockfile. Before publication we packaged that same runtime source as
`hydra-moka 0.12.15-hydra.1`, verified the registry checksum, and replaced the Git source with an
exact crates.io dependency. `Cargo.toml` and `Cargo.lock` now bind the immutable version and
checksum; changing either still requires a new dependency receipt rather than moving a branch or
tag.

The callback does only work that must happen at logical removal time. It computes the already
defined entry-memory delta, decrements the atomic counters, claims a bounded version slot, and uses
`try_send` on a 4,096-ticket channel. It does not await, perform I/O, call back into HydraCache, or
spawn a task per removal. Tag-index cleanup is performed later by ordinary cache operations,
diagnostics, snapshot, or reconciliation drains. A membership is now `(key, entry_version)`, so a
late ticket can remove version 41 without removing version 42.

The queue is bounded, but the production implementation is intentionally not described as
allocation-free before measurement. It uses Tokio's bounded MPSC channel rather than copying the
laboratory ring into the product. The important correctness property is that saturation or a
version-slot collision marks the epoch dirty. Admin snapshots become non-atomic and exact snapshots
return an error until authoritative reconciliation rebuilds tag membership from the live cache.
The local allocation screen must now determine whether this concrete integration preserves the
observer seam's fill-path win and whether mutation costs stay inside the frozen guards.

Adding an eight-byte entry version initially looked like it would invalidate the retained-memory
baseline. We avoided changing the frozen 72-byte inline `CacheEntry` size by replacing the tag
container's 24-byte `Vec` header with a 16-byte boxed slice and using the recovered eight bytes for
the version. This is a useful optimization lesson in miniature: a new correctness field does not
have to become a new retained-memory tax, but the layout claim must be checked by the existing
golden estimator rather than inferred from source.

The most valuable failure happened in a compatibility test. An early integration attached the
observer even when memory instrumentation was `Off`. The cache concurrency matrix then showed a
different capacity-pressure survivor, because merely enabling Moka's removal path can alter backend
maintenance behavior. We changed the builder so `Off` attaches no observer at all. The default path
therefore remains the previous path, while `Production` and the explicit instrumentation profiles
receive exact removal accounting. This is precisely why performance refactors need behavioral tests
that appear unrelated to the target metric.

The admission run covered the full HydraCache test suite, compile-fail UI cases, focused memory,
reclamation, tag-model, cancellation, capacity, replacement, duplicate, saturation, and pending
barrier falsifiers, Clippy in normal and instrumentation-lab configurations, feature leakage,
documentation contracts, and the repository supply-chain policy. A detached worktree at the parent
commit also compiled against crates.io Moka 0.12.15 and passed the previous memory-footprint suite,
so rollback is executable rather than aspirational.

The implementation review found one governance defect rather than hiding it: the preliminary D2
file list named the authorized surfaces but omitted several support files required to implement
them, including `cache.rs`, `entry.rs`, the module declaration, the new observer module, and the
source-policy file. We recorded that variance and the exact changed-file set in a separate admission
receipt before running candidate measurements. The thresholds and public API did not change. This
is another practical reason to separate implementation from measurement: the boundary can still be
audited and corrected without contaminating the candidate result.

At this point local screening is open only as rejection evidence. It can tell us that the integrated
candidate is still too expensive and should return to design. It cannot support a published
numerical improvement. That still requires the admitted Linux host, serialized lease, calibration,
and five counterbalanced pairs frozen in the D3 contract.

### What the integrated local screen found

We did not compare the new candidate only with an old table. We rebuilt the exact pre-observer
commit and the admitted candidate, captured a clean privacy-safe context for each, and ran five
counterbalanced off/production pairs per source on the same local host with the same seed. The two
source campaigns were sequential rather than interleaved, so the result remains a screen, not a
qualification experiment.

The primary allocation result nevertheless became clear enough for a local go/no-go decision:

| Metric | Pre-observer production | Observer production | Change |
| --- | ---: | ---: | ---: |
| Fill allocation per operation | 3,040.38 B | 2,479.31 B | -18.45% |
| Steady-read allocation per operation | 315.19 B | 316.44 B | +0.40% |
| Expire/delete allocation per operation | 3,139.13 B | 2,315.63 B | -26.23% |
| Refill allocation per operation | 1,402.34 B | 1,052.47 B | -24.95% |
| Post-idle RSS growth from cold | 720 KiB | 548 KiB | -23.89% |
| Peak RSS growth from cold | 800 KiB | 644 KiB | -19.50% |

The fill result clears the preregistered 15% local rejection minimum without moving the threshold.
More importantly, the within-candidate off/production medians were 2,480.88 and 2,479.31 B/op: the
old listener's 581.06 B/op fill overhead was no longer visible. That normalized comparison matters
because the absolute off floor moved slightly between source builds.

The mutation result did not come from deleting accounting work. The same candidate passed exact
reconciliation for explicit removal, replacement, expiry, capacity eviction, tag invalidation, and
flush. Expire/delete still cost 48.5 B/op more in production than off, but that is 2.14%, inside the
frozen 3% guard and far below the old 872 B/op listener overhead. Steady reads moved by 1.25 B/op,
also inside both the 3% and 16-byte guard. The candidate's post-idle and peak RSS deltas were 20 KiB
above its off mode, inside the 5%/1 MiB local envelope.

Elapsed time is the least trustworthy part of this screen. Candidate production was 1.5% faster
than the pre-observer production median, while individual within-source pairs varied widely. We
record this as “no local slowdown detected,” not as a speedup. CPU per operation, p99, the 95%
Hodges-Lehmann interval, host calibration, and stable offered-rate windows still belong to the
dedicated D3 run.

This is the desired role of local performance work: the candidate has earned the expensive run,
not the conclusion. Had fill failed the 15% minimum or an allocation/RSS guard, we would have gone
back to the design without consuming dedicated-host time.

### The result changed governance, not just code direction

Before the fork decision, the responsible next action was not to start editing the production cache.
We recorded the owner as D1-classified and explicitly left D2 unauthorized. The proposal registry
prevented candidate measurements and product mutation until four things existed:

- a lab-only feasibility result for the viable backend designs (the sync shortcut is now rejected,
  while the exact nonblocking designs remain to be evaluated);
- a recorded independent or single-maintainer review policy for the selected proposal;
- allocation and RSS rejection limits frozen in an earlier commit before product mutation;
- exact correctness tests for every automatic and explicit removal path.

Those prerequisites are now separated in time and commits. The threshold/review prerequisites and
exact dependency decision are complete, so D2 permits implementation. The correctness prerequisite
now gates the transition from implementation to measurement; it was not silently reclassified as
already passing merely because the dependency itself passed its tests.

We also froze the parts of the measurement method that were already inherited and defensible: at
least five independently started admitted pairs, a 95% interval, Hodges-Lehmann paired estimates,
moving-block bootstrap, Holm correction across primary claims, and the existing 2% goodput and 3%
CPU/p99 regression guards. Allocation and RSS limits first remained named blockers instead of being
chosen from the numbers we had just observed; the later single-maintainer review froze them against
that earlier commit and kept the observer measurements explicitly outside their derivation.

Finally, we created an unadmitted host-profile template. It describes the immutable and mutable host
probes, serialized lease, CPU/NUMA/power policies, and pre/post calibration required for later
qualification. It does not contain a convenient local-machine fingerprint and cannot authorize a
release claim. This separation lets local work continue without allowing local evidence to promote
itself.

The first account-backed admission then converted that template into a real, separately identified
host without converting it into a result. The workflow held a protected serialized lease, captured
the same Linux x86_64 fingerprint before and after a cold release build, and ran five warmed
calibration samples at each boundary. Relative spread was 3.05% before the build and 1.49% after it,
below the preregistered 5% ceiling. Two earlier attempts remain in the ledger: a generic version
probe called `pidstat --version`, which this implementation rejects in favor of `-V`. Treating that
as an unavailable required tool made the lane red before any candidate work. Fixing the probe,
rather than deleting the requirement or silently retrying, demonstrated why admission belongs ahead
of expensive measurement.

Host admission still did not authorize D3. It proved that the machine, toolchain, affinity policy,
lease and short calibration boundary were reproducible. It did not yet establish stable offered
rates or measurement-window length for I73. Those values must come from baseline-only pilots; only
after they are frozen can the observer candidate be observed on this host. This is another useful
separation: qualifying the laboratory is not the same as accepting an experiment performed in it.

The first baseline-only pilot then produced exactly the kind of inconvenient result this separation
was designed to preserve. Across four offered rates, all 24 processes completed without an outcome
or reconciliation failure. Both modes delivered at least 99.96% of offered load, p99 stayed below
two milliseconds, and goodput regression was effectively zero. On those dimensions the selected
windows looked comfortably stable.

The run still failed its preregistered decision rule. Production instrumentation consumed 4.14%,
6.46%, 10.01%, and 5.27% more CPU per operation at 2,500, 5,000, 10,000, and 20,000 operations per
second. The aggregate result was not created by one bad process: production used more CPU in all 12
counterbalanced pairs, whether it ran first or second. Expressed as an absolute cost, the observed
paired deltas ranged from roughly 0.08 to 0.62 microseconds per operation. Allocation supplied a
second repeatable signal: production added about 19.4-20.4 bytes per operation at every rate.

This is a useful example of why “the server kept up” is not the same statement as “the baseline is
acceptable.” An open-loop workload can meet its offered rate while spending more CPU headroom to do
so. That headroom matters before saturation and is exactly what the independent CPU gate protects.
Because the 3% ceiling was frozen before the run, zero rates qualified and I73 remained unfrozen.
The failed workflow is therefore retained evidence, not infrastructure noise and not permission to
raise the limit.

The right follow-up is narrower than repeating the entire scan. The existing laboratory seams can
separate four configurations: instrumentation off; counters with backend removal observation
disabled; a registered backend observer with an empty callback; and complete production accounting.
Their adjacent differences attribute the cost respectively to counters, backend notification
plumbing, and HydraCache cleanup/accounting work. That diagnostic remains non-promotable because two
configurations deliberately break removal correctness. Its job is to tell us where to optimize; the
unchanged off-versus-production contract must still make the eventual acceptance decision.

The four-mode run made that distinction concrete. Counter-only instrumentation measured 0.20%
below off, so the experiment detected no CPU cost attributable to the counters themselves. Adding
an empty post-removal observer increased median CPU per operation by 1.42%; replacing it with the
complete callback and cleanup path added another 1.99%. End to end, production was 3.24% above off,
with about 20.23 additional allocated bytes per operation. All 20 processes completed, and the
host's 4.27%/2.09% pre/post calibration spreads stayed inside the frozen 5% envelope.

This result changes the implementation question from “are atomics expensive?” to “why do calls
with no cleanup work still enter async coordination?” HydraCache asks the observer to drain before
and after many mutations. Most of those calls find an empty channel, yet they still acquire the
receiver's async mutex. The observer already maintains accepted and acknowledged sequence counters,
so equality provides a cheap no-work hint. Checking that hint before locking is race-safe: the old
implementation could also observe an empty receiver immediately before a concurrent publication,
and every exact snapshot and later mutation drains again or fails closed on unequal sequences. The
optimization can therefore remove repeated empty locks without weakening delivery, version checks,
saturation handling, or exact reconciliation.

That change was deliberately small. The drain path first loads the accepted and acknowledged
sequences with acquire ordering and returns when they are equal. It does not advance either
sequence, consume a ticket speculatively, or treat equality as proof for a later snapshot. Tests
instrument the lock acquisition itself: an empty observer takes no lock, a published removal takes
exactly one and reaches a clean acknowledged state, and the next empty drain again takes none. The
broader observer and memory-accounting suites then re-prove duplicate handling, saturation,
version-conditional tag cleanup, capacity eviction, expiry, and exact reconciliation. This is the
kind of optimization a local machine can close completely at the mechanism/correctness layer before
spending another admitted-host run on the quantitative question.

The next unchanged baseline run showed both the value and the limit of that tactic. CPU overhead at
10,000 and 20,000 operations per second fell inside the 3% ceiling, to 2.80% and 2.08%. At 2,500 and
5,000 it was still 3.52%. Two rates therefore passed, but the preregistered rule required three.
Calling that “close enough” would erase the purpose of the rate grid: fixed coordination costs are
most visible when useful work is sparse.

That shape points to the remaining coordination primitive. A Tokio bounded channel is appropriate
when producers and consumers need asynchronous waiting; this callback is forbidden to wait, uses
`try_send`, and is drained opportunistically by a caller already in async context. A bounded
lock-free array queue better matches those semantics: fixed capacity, no per-ticket node allocation,
nonblocking push, and direct pop without an async receiver mutex. The dependency was already in the
resolved graph, but making it direct still requires an explicit supply-chain check. Most
importantly, changing the container must not change the protocol around it: slot-based duplicate
detection, dirty-on-overflow, accepted/acknowledged barriers, version-conditional tag cleanup, and
reconciliation recovery remain the actual correctness contract.

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

## What the first dedicated runs changed

The first admitted-host baseline did not validate the production observer. All four offered rates
exceeded the preregistered 3% CPU-per-operation ceiling, even though every attempt completed and the
host stayed inside its calibration envelope. A four-mode attribution run then split the cost into
three layers: counters alone had no detected CPU penalty, registering an empty backend observer
added about 1.4%, and HydraCache's real callback and cleanup added about another 2.0%. That result
gave us an owner, not permission to relax the ceiling.

The first code change removed an async receiver-lock acquisition from empty drains. Repeating the
same baseline contract moved the 10,000 and 20,000 operations/second points below the ceiling, at
2.80% and 2.08%, but the 2,500 and 5,000 points both remained at about 3.52%. Two stable rates were
progress; the frozen contract required three. We retained the failed campaign and did not average
the passing high-load points into an acceptance claim.

The shape of the result matters. When a percentage penalty falls as offered load rises, a fixed
per-drain or synchronization cost is a stronger suspect than work proportional to every request.
That is an inference to test, not proof. It led to a second, narrowly preregistered change: replace
the bounded Tokio channel and async receiver mutex with a preallocated bounded `ArrayQueue` and an
atomic single-consumer claim.

The implementation preserves the safety properties that performance work is most likely to erode:

- capacity remains 4,096 tickets rather than becoming an unbounded queue;
- publication still cannot await or block;
- a full queue or version-slot collision marks the observer dirty;
- accepted and acknowledged sequences remain the exactness barrier;
- delayed cleanup remains conditional on entry version;
- reconciliation still repairs a dirty epoch;
- an RAII guard releases the consumer claim if a drain future is dropped.

Local tests establish those properties, but they do not establish that the change is faster. Even
an intuitively cheaper primitive can lose because of cache-line contention, retry behavior, or a
different workload mix. The next legitimate performance statement must therefore come from another
unchanged baseline-only run on the admitted host. If it still fails, the failure remains evidence;
if at least three rates pass, only then may the release freeze its integrated baseline and proceed
to candidate qualification.

That repeat produced another useful rejection. The two high rates improved to 2.48% and 0.99% CPU
overhead, but 5,000 operations/second remained just outside the ceiling at 3.23%, and 2,500 measured
7.04%. All 24 attempts completed, throughput and p99 did not regress, and host calibration was
tighter than 0.3%, so this was not dismissed as an infrastructure failure. The `ArrayQueue` change
was retained for its simpler bounded publication path and green correctness proof, but the run did
not establish a performance improvement: cross-run differences are diagnostic, not paired evidence.

The next suspected cost is smaller and more mechanical. Both publication and acknowledgement use a
checked atomic update expressed as a compare-and-swap loop. Replacing each with one `fetch_add` can
remove retries and branching while still detecting wrap from the returned previous value. The
important constraint is semantic: overflow must still make the observer dirty and exact snapshots
must remain unavailable until reconciliation. This optimization is preregistered and tested locally
before another host minute is spent.

The implementation made the atomic operation itself the sequence linearization point. If
`fetch_add` returns `u64::MAX`, the counter has wrapped, but the observer marks the epoch dirty before
returning. Exactness checks consult that dirty bit before comparing the counters, so two equal zeros
cannot masquerade as a clean state. A forced-overflow test covers both publication and
acknowledgement and verifies that reconciliation is the only recovery path. This is a useful pattern
for micro-optimization: remove mechanism, not the invariant that mechanism was protecting.

The next dedicated repeat invalidated a different assumption: the pilot itself was not repeatable
enough to steer another micro-optimization. At 5,000 operations/second, successive admitted runs
reported 3.23% and 10.13% CPU overhead; at 10,000 they reported 2.48% and 5.46%. The host calibration
spread stayed below 1.21%, every operation completed, and the intervening code change only replaced
two atomic increment loops. Calling that change a regression would be as unjustified as calling the
earlier result an improvement.

Counterbalancing alone does not make an estimator paired. The first pilot launched off and
production in alternating order, but then calculated a median for each mode independently and took
their ratio. With three short pairs, one expensive production process could move the decision while
its adjacent control observation was discarded as a pair. The corrected pilot keeps the same
workload, rates, and ceilings, but uses five ten-second pairs and applies Hodges-Lehmann to the five
within-pair differences. This increases measurement volume; it does not widen the acceptance gate.

We also removed the workflow's push trigger. Dedicated-host qualification is now manual-only behind
the protected environment. Cheap correctness and contract checks still run locally, while a normal
documentation or implementation push cannot accidentally start an expensive campaign.

The tool enforces those choices rather than relying on operator memory. It refuses fewer than five
pairs, shorter than ten-second windows, a changed rate grid, warmup, or seed. Every raw attempt still
has its own directory and digest. The aggregate carries both the per-pair deltas and their
Hodges-Lehmann estimate, so a reviewer can reconstruct the decision and see whether a single pair
was influential. Improving measurement here is part of the optimization: it prevents us from
spending code complexity on a fluctuation the benchmark cannot reproduce.

The first strengthened run still rejected the baseline, but with a much clearer shape. Five paired
ten-second observations produced Hodges-Lehmann CPU overhead estimates of 3.85%, 1.23%, 3.86%, and
0.94%. Goodput and p99 passed, all 40 processes completed, and calibration stayed within 1.44%.
Only two rates passed. This is stronger evidence than the earlier short runs: the residual cost is
small, workload-dependent, and still real enough to keep I73 unfrozen.

The paired samples also prevent overreacting to one anomaly. One 10,000-rate pair measured 13.61%
CPU overhead, while the other four measured roughly 1.99%, 2.26%, 3.86%, and 3.94%. The robust
estimate was 3.86%. Removing the outlier because it is inconvenient would be wrong; letting it
single-handedly set the answer would also be wrong. Retaining both the samples and the estimator
makes that distinction reviewable.

The next code target follows directly from attribution and code structure. Each removal updates
eight retained-memory counters, and every update is a checked compare-and-swap loop. Those counters
are already protected by an active-mutation epoch. A single `fetch_add` or `fetch_sub` can detect
wrap from its returned old value and set the fault bit before the guard releases quiescence. That
removes retry loops without weakening exact snapshots. The active-mutation, version, and epoch
algorithms remain unchanged because their overflow windows have different synchronization risks.

We implemented that narrower change and resisted the tempting global rewrite. The eight data
counters now perform one atomic read-modify-write each. Overflow and underflow do modify the raw
counter by wrapping, unlike the old failed compare-and-swap update, but that value is never allowed
to become evidence: the same operation's old value exposes the fault, the permanent fault flag is
published while the mutation is still active, and both barrier and capture reject the subsystem
before reading it as exact. There is no recovery path that silently blesses the wrapped number.

This distinction matters. A data counter lives inside a wider mutation protocol, so fail-closed
wrap detection after its linearization point is sufficient. The active-mutation count, version,
and epoch *are* that protocol; changing them to wrapping operations would create different windows
where quiescence or snapshot identity could be misreported. Similar-looking atomics therefore do
not automatically have the same safe optimization. We removed retry machinery only where the
surrounding invariant already supplied the safety boundary.

The local proof deliberately targets the dangerous ordering, not just the happy path. Tests hold
the mutation guard, force overflow or underflow, and ask for a barrier before releasing the guard.
Receiving `CounterFault` instead of `NotQuiescent` demonstrates that the permanent fault became
visible first. Capture is rejected while the guard is active, and the barrier remains rejected
after it drops. Existing reconciliation and memory-accounting suites then show that ordinary
insert, replace, delete, expiry, and flush behavior did not move.

That is still not a performance result. Local tests prove that the proposed cheaper mechanism
preserves failure semantics; they cannot prove that fewer possible CAS retries reduce CPU on the
reference host. The receipt therefore authorizes exactly one unchanged manual v2 baseline repeat.
If three rates pass, we can freeze I73. If they do not, the packet becomes another retained
falsifier instead of an invitation to move the threshold.

The repeat did not pass. With the single-atomic counter implementation, paired CPU overhead was
3.53%, 4.69%, 4.44%, and 1.80% across the four rates; only 20,000 operations/second stayed below the
3% ceiling. All 40 attempts completed, goodput remained essentially unchanged, and calibration
spread stayed below 2.68%. This is a valid negative measurement, not an infrastructure failure.

It would also be incorrect to call the counter change a regression. The preceding V2 campaign had
two stable rates, but the old and new binaries were measured in separate campaigns rather than as
paired treatments inside one schedule. Cross-run movement describes repeatability; it does not
isolate causality. The new campaign alone is enough to reject the baseline freeze, while the code
change remains acceptable only on its independently proven simplicity and correctness merits.

One signal repeated across every cell: production allocated roughly 20.1--20.5 more bytes per
operation than off. Unlike the CPU estimate, that delta barely changed with offered rate. It does
not prove that allocation causes all CPU overhead, but it gives the next local investigation a
specific target. We stop spending reference-host minutes and return to allocation attribution:
observer delivery, queue publication, cleanup, and tag-index work must be separated before another
product optimization is selected.

The local split found the owner without another bare-metal campaign. Counter updates changed gross
allocation by only a few bytes per operation, and get-only traffic was neutral. Enabling an empty
post-removal observer, however, added about 79 B/op to replacement, 81 B/op to remove/refill,
118 B/op to tag invalidation/refill, and 76 B/op to TTL puts. The complete production callback did
not add the positive adjacent delta; remove and tag invalidation retained +76 and +86 B/op versus
off, which combined into +24.80 B/op in the original mixed workload.

The code explains the shape. Moka must deliver an owned value to the observer and calls
`entry.value.clone()`. HydraCache's value derives `Clone` and stores tags as `Box<[String]>`, so
cloning a removed entry copies the slice and every tag string before the callback can move that copy
into its bounded cleanup ticket. Reads do not invoke the observer, and counters do not clone the
entry, matching the two neutral ablations. This is stronger attribution than merely noticing that
allocations correlate with CPU: the metric, operation family, ablation boundary, and source-level
ownership all agree.

The next optimization therefore changes ownership rather than shaving another atomic instruction.
Sharing immutable tags between the stored entry, Moka's observer clone, and the cleanup ticket can
turn the deep clone into a refcount increment. That proposal still needs its own safety contract:
versioned tag cleanup, bounded queue behavior, retained-memory estimates, and public APIs must stay
unchanged, and the exact 120-process local matrix must show that the identified allocation moved
before any dedicated-host run is considered.

That ownership change produced the local result the hypothesis predicted. The entry and cleanup
ticket now share immutable tags through `Arc<[String]>`. Moka can still clone and deliver an owned
entry, but cloning the tag field increments a reference count instead of allocating a new slice and
copying every string. Public events deliberately remain unchanged: when an event is actually
published, HydraCache materializes owned strings at that boundary.

The important comparison is not the absolute allocation level, which also includes the workload
and allocator, but the same adjacent ablation boundary before and after the change. Replacement
moved from +78.62 to +1.68 B/op when enabling the noop observer; remove/refill moved from +81.02 to
-1.94 B/op; tag invalidation/refill moved from +117.85 to -6.47 B/op; and TTL puts moved from
+76.03 to +0.24 B/op. Reads remained neutral. In other words, the allocation appeared exactly when
observer delivery began before the change and disappeared at that boundary after ownership became
shared.

Production results add a useful nuance. Remove/refill fell from +76.17 to -1.99 B/op relative to
off, while tag invalidation/refill fell from +85.71 to +14.83 B/op. The remaining positive tag cost
is downstream of the noop observer boundary and is consistent with constructing the intentionally
owned public event payload. We do not call negative local deltas speedups: independent short
processes and allocator reuse can move medians by tens of bytes. The defensible claim is narrower
and stronger—the 76--118 B/op deep-clone owner was removed, and the remaining allocation belongs to
a different, explicit API boundary.

This is also the point where a local profiler has done its job. It selected a candidate, predicted
which cells should move, and falsified the unwanted ownership pattern without consuming another
reference-host campaign. It still cannot establish CPU overhead. The next expensive run is now
justified as confirmation of a specific mechanism, so the evidence receipt permits one unchanged,
manual paired V2 repeat and no automatic or post-result retry.

The confirmation passed decisively. The exact-source reference-host run completed all 40 attempts,
with pre/post calibration spread of 0.20% and 0.44%. All four offered rates were stable. Paired CPU
overhead was 0.052%, 0.090%, 0.167%, and 0.594% from 2,500 through 20,000 operations per second;
goodput and p99 stayed inside their unchanged guards. Production allocation was consistently about
26 B/op below off, reversing the earlier roughly +20 B/op signal without changing the workload or
thresholds.

The local and hosted results answer different questions. The 120-process local ablation explains
*why* the allocation moved: the noop-observer boundary lost its deep tag clone. The dedicated-host
paired campaign establishes that the integrated production instrumentation now fits its frozen CPU,
latency, allocation, and stability budget. Neither result should be stretched into a claim that this
commit caused a particular CPU improvement versus an older campaign; those campaigns were not a
paired code comparison. Together they are sufficient for the intended governance decision: freeze
`I73` at the measured source, select the 20,000 ops/s knee, derive 5,000/12,000/17,000 ops/s D3
cells, and permit candidate observation only under separately preregistered comparisons.

The next source audit also changed how we plan instrumentation. The cache already has an exact
retained-byte snapshot split by entries, values, keys, tags, tag-index memberships, expiry metadata,
and generation metadata. HC/2 already reconciles live connections, invocations, subscriptions, and
sessions; the durable store exposes logical bytes and GC reclamation; management paths already have
hard concurrency and response bounds. Adding parallel counters for those owners would increase the
cost of the baseline while producing duplicate evidence.

The real gaps are narrower: lock wait and cloned expiry keys in the shared client store; copied
bytes at RESP and HC/2 boundaries; queued HC/2 bytes; management collector wakeups and retained
cursor/history bytes; durable staging buffers versus file-backed pages; and allocator
allocated/active/resident/retained state. Some of those belong in focused test-only or production
counters, some in allocation probes, and some only in OS or allocator-native telemetry. Treating
every unknown as a new hot-path counter would confuse observability with ownership—and could recreate
the overhead problem we just removed.

The first follow-up probe demonstrates why the measurement layer matters. The shared client store's
bounded expiry sweep used an owned `Vec<StoreKey>` for every examined entry, then cloned expired
keys into a second vector and cloned the final examined key once more into the cursor. Because a
`StoreKey` contains three owned strings, this is not a cheap tuple copy. It is three allocations and
all identity bytes copied for every clone, while the store mutex remains held.

A profile-only feature called the real sweep from a standalone release binary; it added no counters
or timers to the frozen production path. The fixed fixture contained 512 entries, a 256-entry scan
budget, and 96 identity bytes per tuple key. Five independent processes per shape produced identical
results. With no expired entries, the sweep made 772 allocations and allocated 43,104 gross bytes.
Of those bytes, 18,432 are the reserved 256-element examined vector and 24,672 are the three strings
in 256 examined-key clones plus one cursor clone. Crossing the ordered-map boundary produced exactly
the same result, which is evidence that wraparound itself is not a separate owner.

When half the examined entries were expired, the result rose to 1,162 allocations and 73,536 bytes;
when all were expired, it reached 1,547 allocations and 104,256 bytes. The increase is explained by
the necessary owned removal keys plus geometric growth of the filtered expired vector. Gross
allocation deliberately counts the full destination size of every reallocation, so this is churn,
not retained memory. The decomposition predicts every observed byte: it is a much stronger basis for
a change than an RSS correlation.

The candidate boundary is now narrow. Removal still needs owned expired keys because the map is
mutated after scanning, and bounded progress still needs one owned cursor. The examined keys need
neither. A borrowed scan can retain only expired tuples and the final cursor, reducing the fixed
43,104-byte no-expiry cost to the cursor's 96 bytes while preserving the 256-entry budget, wrap
semantics, and quota cleanup.
We do not yet claim shorter lock duration: the allocation is under the mutex, so that outcome is
plausible, but it requires a separate measurement rather than inference.

The candidate matched that byte model exactly. In all four shapes it removed 43,008 gross bytes,
769 allocations, and 24,576 copied identity bytes per bounded scan. No-expiry and cursor-wrap both
fell from 43,104 bytes and 772 allocations to 96 bytes and three allocations: precisely one owned
three-string cursor. Half-expired fell from 73,536 to 30,528 bytes, and all-expired from 104,256 to
61,248 bytes. Five fresh processes per shape returned identical results.

The percentage is deliberately secondary to the decomposition. Depending on how many keys expired,
the reduction is 99.78%, 58.49%, or 41.25%, but the removed owner is invariant. Correctness tests
also retained the scan budget, explicit wraparound, full sweep without a cursor, empty-store cursor
cleanup, and tenant quota release. That combination lets us accept W2 locally without buying a
dedicated-host run. CPU and mutex-wait claims remain open; W2 will join a later integrated candidate
rather than receiving a bespoke expensive campaign.

The next owner appeared at a transport boundary. HC/2 already represents the mutation key and value
as immutable reference-counted `Bytes`, but event fan-out converted them back to slices and called
`Bytes::copy_from_slice` for every matching subscription. The outbound channel is bounded and awaits
capacity, so this is not an unbounded queue bug. It is bounded amplification: every queued event owns
another complete key and value allocation.

A standalone allocation probe reproduced only those two constructor calls, with its frame vector
reserved before measurement. The result was exact in 20 independent processes: two allocations per
subscriber and gross bytes equal to `fanout × (key bytes + value bytes)`. A 64-byte key plus 128-byte
value cost 192 bytes at fan-out one, 1,536 bytes at fan-out eight, and 3,072 bytes at fan-out sixteen.
With a 4,096-byte value and sixteen subscribers, one event created 66,560 bytes of payload copies
before protobuf, HTTP/2, or TLS buffering was considered.

This decomposition keeps the candidate small. Passing `Bytes` through the fan-out function and
shallow-cloning it into each immutable wire message should remove the payload allocations while the
last queued event still owns the backing storage. Ordering, watermarks, bounded `send().await`
backpressure, and disconnect reconciliation remain the falsifiers. Tonic and TLS buffer ownership is
a separate W5 subproblem and must not be inferred from this result.

The first candidate probe correctly failed its zero-allocation contract. Every shape reported two
allocations and 48 bytes, independent of fan-out and payload size. The invariant exposed a fixture
error: cloning `Bytes` created directly from a unique `Vec` performs a one-time promotion to shared
ownership. The real server already performs that promotion in `mutation_event`, before entering the
fan-out function. The red result is retained; the fixture must reproduce that ownership state before
we can judge the per-subscriber clones.

The corrected fixture retained both owners before measurement, exactly as the server does after
`mutation_event`. Under that preregistered rerun, all 20 processes reported zero allocation in the
fan-out window while retaining every frame. An actual server test additionally verified that two
event frames preserved their bytes and shared the original backing pointers; the real-mTLS test
continued to deliver the event and reconcile all connection-owned resources to zero after close.
Thus the event-copy subproblem is accepted locally. W5 itself remains open for channel storage,
connection-task allocation, and transport buffers.

There is a broader lesson in the failed 48-byte run. Two values can have the same Rust type and
different allocation behavior because their internal ownership state differs. A `Bytes` created
from a unique `Vec` has not yet paid the transition to shared ownership; a `Bytes` cloned from a
decoded request has. A benchmark that recreates the type but not the ownership history can charge a
setup transition to the operation under test. The corrected fixture therefore retained both the
original and cloned owners before opening the allocation window. This was not a statistical
adjustment: the threshold stayed at zero, the failed packet remained in the ledger, and the new
fixture encoded a source-level fact that can be falsified.

The queue model also constrains what we can claim. Each HC/2 connection has a bounded Tokio channel
whose capacity equals `max_streams_per_connection` (16 by default), and `send().await` stops the
single connection producer when that channel is full. A blocked send can retain its producer-held
frame in addition to queued frames. Sharing payload buffers removes multiplicative copies across
those frames, but it does not remove the channel slots, protobuf encode buffers, HTTP/2 flow-control
state, TLS records, or the connection task itself. Those owners need a connection census with exact
logical reconciliation before process memory can be divided by connection count.

That census used the real mTLS listener and client, not a mocked channel. Groups of 1, 10, 100, and
1,000 clients were opened in sequence. At every plateau, `active_connections` equaled the requested
cardinality and `accepted_connections - closed_connections` equaled the live count; subscriptions,
sessions, and pending invocations stayed at zero. Every client also reported empty pending maps and
restored permits. After each group closed, accepted equaled closed and every server-side live owner
returned to zero. The 1,000-connection group completed inside the same local test in 6.69 seconds.

This is a denominator, not a memory result. We now know that a process-level delta at 1,000 clients
cannot be explained by silently missing or already-closed logical connections. We still cannot divide
RSS by 1,000 and call the quotient a connection cost: allocator arenas, shared TLS state, code pages,
HTTP/2 buffers, task stacks, and nonlinear capacity growth must be separated through independent
processes and multiple cardinalities.

The independent-process profile then supplied the missing shape, while also showing why that warning
matters. Across three fresh processes at each cardinality, cumulative allocation during connection
creation was almost perfectly linear: the median stayed between 278,280 and 278,316 gross bytes per
local client/server pair from one through 1,000 connections. That number is allocation churn, not
retained memory. It includes both endpoints, certificate and TLS setup, HTTP/2 state, runtime work,
and anything freed before the plateau. Its stability makes it a useful decomposition target, but it
does not make it a server-side bytes-per-connection result.

Working set told a different story. Median plateau deltas were 2,699,264 bytes at one connection,
4,198,400 at ten, 15,675,392 at one hundred, and 122,925,056 at one thousand. The marginal change
from 100 to 1,000 was about 119,166 bytes per additional local pair, far below the gross-allocation
figure and different from the older D0 slope. That is expected: gross allocation counts churn, while
working set is a noisy retained-page snapshot with fixed process warm-up, allocator size classes,
shared runtime state, and both sides of the loopback connection. A single straight line through all
four points would hide the fixed floor and manufacture false precision.

After close, median working set still sat 2,748,416, 3,489,792, 4,358,144, and 6,656,000 bytes above
the corresponding baselines. Yet accepted equaled closed, the server reported no live connection,
invocation, subscription, or session owner, and every client map and permit set reconciled. The
residual is therefore evidence of process high-water behavior, not evidence of a logical leak. It
may be reusable allocator or runtime capacity, and W8 must test that reuse explicitly before anyone
proposes trimming it.

This local result closes one question and opens a narrower one. Scaling is material enough to justify
separating the endpoints, but the combined process cannot tell us whether the dominant owner is the
client, HydraCache server state, tonic/HTTP2, TLS, or allocator bookkeeping. The next probe must keep
the same protocol and cardinalities while placing server and clients in different processes and
sampling both. Only then is it responsible to choose among initial-buffer sizing, queue-byte caps,
or idle buffer release; RSS alone still does not authorize any of them.

Separating the endpoints made the decomposition much sharper. Over the 100-to-1,000 interval, the
client process contributed about 181,324 gross allocation bytes and 70,529 working-set bytes per
additional connection; the server contributed about 97,023 gross allocation bytes and 49,625
working-set bytes. The allocation slopes sum to 278,348 bytes per pair, only 0.024% away from the
combined-process slope. The working-set slopes sum to 120,154 bytes, 0.83% above the combined result.
Those close reconstructions are a useful consistency check: process separation changed fixed floors,
but did not invent a different scaling phenomenon.

The server-only working-set slope is especially informative. Its local 49,625 bytes per connection
is 7.48% above the older D0 observation of 46,171, despite a different harness and measurement date.
That resemblance increases confidence that the original signal was real, while the difference is a
warning against turning either number into a universal constant. Client process state accounts for
roughly 65.1% of allocation churn and 58.7% of split working-set growth, but it was absent from the
old server RSS slope. Endpoint scope explains why both measurements can be correct.

Even the server number is not yet an application-owner number. One process contains the HydraCache
subscription/session maps and bounded channel, but also tonic, HTTP/2 flow control, rustls records,
Tokio tasks, socket buffers charged into the process, and allocator bookkeeping. The idle matrix
therefore accepts the endpoint attribution without authorizing a buffer change. The next frozen cell
is one hundred slow consumers: it can test the bounded queue/frame-retention model and exact cleanup,
which is the remaining application-level W5 hypothesis, while leaving generic TLS tuning alone.

The slow-consumer fixture itself needed correction before it could answer that question. The older
helper opened one subscription on each of one hundred clients but issued all 1,100 mutations through
the first client. HC/2 subscription maps are stream-local, so ninety-nine nominal slow consumers saw
no events. The corrected profile gave every connection a unique prefix and made every client produce
1,100 matching mutations. Both drained and unread sides therefore completed the same 110,000 real
mTLS mutations while retaining exactly one hundred active subscriptions.

Five counterbalanced process pairs isolated the unread receiver. Relative to continuously drained
receivers, it added a median 20,037,582 gross allocation bytes and 21,204,992 working-set bytes in
the client process. The paired server medians were -1,829 allocation bytes and -57,344 working-set
bytes: noise around zero under the same mutation load. All controls delivered 110,000 events with
zero drops. Every unread run accepted exactly 51,300 events and recorded 117,493 to 117,495 drops.
The repeatability and endpoint separation identify the retained owner much more strongly than a
whole-process RSS difference could.

The apparently surprising 51,300 count follows the protocol model. The server watermark is global,
while each unique-prefix subscription sees only its matching mutations, so gap notifications share
the client's 1,024-item subscription queue with events. Once that bounded queue fills, subsequent
gap/event delivery attempts are rejected and coalesced into repair state instead of growing memory
without limit. This is honest bounded degradation: the application must repair after the gap, but
the transport reader and server are not blocked by an application that stopped calling `next()`.

That last distinction prevents another overclaim. This experiment closes the normal application
slow-consumer hypothesis as client-owned and bounded; it does not simulate a hostile peer that stops
polling the HTTP/2 response stream itself. The server still has a 16-item outbound channel, but an
item limit is not a byte limit when frames vary in size. A raw transport-stall probe is the next
legitimate way to decide whether byte admission is needed. The 21 MB client result cannot be used as
evidence for changing the server queue.

The raw probe made that hostile boundary real without changing product counters. Eight wire-level
clients completed mTLS, handshake, and subscription acknowledgement. Controls kept polling the
response stream; treatments retained the same `Streaming` objects and request senders but made no
response poll during the pressure window. Each peer offered exactly 8 MiB of values in both cells:
2,048 mutations at 4 KiB or 32 mutations at 256 KiB. A server-side dispatch counter sampled after a
two-second settle and again 500 milliseconds later proved that every treatment had actually stopped,
while every control completed its full volume.

Across five counterbalanced pairs per payload, unpolled-minus-drained server working-set deltas were
positive in all ten pairs. Their median rose from 9,216,000 bytes at 4 KiB to 22,667,264 bytes at
256 KiB; pagefile medians moved from 9,461,760 to 22,663,168 bytes. This is repeatable payload-size
sensitivity, but still a process-level result. It does not tell us that those bytes all occupy the
Tokio channel: protobuf encoding, Hyper/h2 send state, rustls, socket handoff, and allocator pages
remain in the same server process.

Source and measurement together give a stronger boundedness model. The locked Hyper 1.11.1 client
uses a 2 MiB default stream receive window and a 5 MiB connection window. Median stalled dispatch
was 4,160 mutations in the 4 KiB cell, or 520 per peer. That is approximately 512 value-bearing
events admitted by a 2 MiB stream window plus a small bounded tail. At 256 KiB, the median was 128,
or sixteen per peer: approximately eight events in the stream window plus a similar tail. Invocation
responses and global-watermark gap frames share the path, so this is a source-supported decomposition,
not an exact queue-occupancy equation. It does explain why an item bound alone cannot express the
memory exposure.

The fixture produced two useful red smoke tests before the frozen matrix. The first classified
legitimate gap frames as unexpected; the second showed that merely dropping a stalled tonic stream
did not reconcile quickly enough for a deterministic close proof. The final fixture counts gaps,
takes its pressure snapshot while the stream remains completely unpolled, then resumes reading only
to drain the already offered frames before closing. Snapshot response counts remain zero, and every
run finishes with accepted equal to closed and all server-owned live resources at zero. Neither red
smoke result was silently replaced in the twenty-attempt matrix.

This evidence is enough to preregister an application outbound-byte-admission candidate, not to
declare it successful. The candidate must retain the existing 16-item limit, charge the encoded
`ServerEnvelope` while it waits in the application queue, avoid deadlock for one legal frame larger
than the byte budget, and release every permit on send failure or disconnect. Its claim must stop at
the point where tonic polls the item: HTTP/2, TLS, and socket retention are downstream owners and
need their own controls. That narrower statement turns “add a byte cap” from a slogan into a
falsifiable ownership change.

The candidate used a one-MiB semaphore per connection and charged each `ServerEnvelope` by its
protobuf encoded length before the existing sixteen-item channel. The queued wrapper owns the
weighted permit until tonic polls that item. A legal envelope larger than the budget consumes the
whole budget rather than being rejected, so one oversize frame can always progress; the single
connection producer means only one already-constructed frame can wait outside the admitted queue.
This preserves protocol semantics while making the application-owned queue express both an item and
a byte bound.

The correctness proof targets the awkward edges, not only the happy path. Unit tests observe the
exact encoded charge, show a second oversize frame blocking, then prove permit release on poll, on a
closed receiver, and on disconnect. The real-mTLS test still preserves event ordering and shared
payload bytes and reconciles connections, subscriptions, sessions, and invocations to zero. The
complete server suite and strict clippy gate also pass; no public configuration, client API, or wire
contract changed.

Repeating the frozen twenty-process matrix separated the two active bounds. At 4 KiB, median stalled
dispatch barely moved, from 4,160 to 4,152 total mutations, because the sixteen-item limit remains
tighter than one MiB. Its paired working-set ranges overlapped, and the median moved from 9,216,000
to 9,863,168 bytes; this is not a small-frame memory win. At 256 KiB, median stalled dispatch fell
from 128 to 104, exactly the direction expected when the application tail changes from roughly eight
value-bearing frames per peer to roughly four plus the producer-held frame.

The high-payload retained-memory result moved with that dispatch bound. Median unpolled-minus-drained
server working set fell from 22,667,264 to 15,949,824 bytes, a local reduction of 6,717,440 bytes or
29.6%. Pagefile fell from 22,663,168 to 15,167,488 bytes, or 33.1%. Every one of the five candidate
pairs was below every one of the five retained baseline pairs. All controls completed, all treatments
were stable for the frozen interval with zero response polls, and every process closed eight of eight
connections with no live server resources.

The remaining roughly sixteen megabytes are as informative as the reduction. The new admission gate
cannot reclaim a frame already polled by tonic, Hyper/h2's peer-advertised receive window, protobuf or
TLS work buffers, socket ownership, or allocator high-water pages. The candidate is therefore
accepted locally as an application-queue bound and retained for the later integrated W5/W2 campaign;
the percentages are diagnostic, not release claims, and do not justify a dedicated-host run for this
subproblem alone.

The remaining Wave A surface was management overhead. Here the source audit prevented us from
building the wrong benchmark. Management aggregation is not a periodic collector: it runs when a
formation, consensus, health, or related snapshot is requested, and its one-second cache is populated
by that request. The optional Prometheus history adapter is request-scoped as well. The management
source files contain no `spawn` or `interval` loop. A generic “server idle” comparison would mostly
measure the common expiry-maintenance task and Tokio runtime, then incorrectly charge them to the
management API.

We therefore split W6 into seven ownership cells in one standalone release binary. Five
counterbalanced process pairs compared construction with management routes off and on. Five fresh
processes each then measured sixty dashboard reads at one read per second, sixty cold aggregate
refreshes, sixty hits against one retained aggregate, cursor saturation, and sixty valid history
reads with the adapter disabled. Every process reported gross allocation, process-wide live
allocation, working set, pagefile, response bytes, elapsed time, transport calls, and logical cursor
cardinality. The full matrix completed 35 of 35 attempts with empty stderr and every route, cache,
cursor, and history falsifier green.

Mounting the management route graph added a median 95,261 gross allocation bytes and 21,797 live
allocator bytes relative to the disabled route graph. Those numbers describe one-time construction,
not a per-second tax. No idle process made an aggregate transport call, a disabled surface returned
404 for the management route, and the source audit found no management-owned background task. This
closes the feared “collector wakes even when nobody is reading” branch: there is no collector to
optimize. Lazily replacing a small one-time route cost would add lifecycle complexity without
addressing a recurring owner.

The ordinary polling cells also put their scale in context. Sixty dashboard reads allocated exactly
927,564 gross bytes in every repeat, or 15,459.4 bytes per read, while serializing 1,901 bytes per
response. At one read per second that is roughly 15 KiB/s of allocation churn, not retained growth.
A disabled-history read allocated 8,942 bytes and returned 351 bytes; its process-wide live delta was
zero in every repeat, and every response said `no_adapter`. Constructing an upstream client or
running DNS while history is disabled would have failed this cell, but neither happened.

The aggregate cache, by contrast, proved that an existing optimization is doing real work. Sixty
cold refreshes caused exactly sixty transport calls and a median 599,684 gross allocated bytes.
Sixty reads of the same valid snapshot caused one transport call and 362,949 bytes. Cache reuse
therefore removed about 39.5% of gross allocation and 32.0% of elapsed time in this local fixture,
while returning the same 29,400 serialized bytes. This is not evidence for a new candidate; it is
evidence to preserve the current epoch-, observation-, roster-, and TTL-bound cache semantics.

Cursor saturation shows why logical bounds and allocator snapshots must be reported separately. The
fixture issued 1,025 truncated formation pages against a 250-member snapshot, then retried the first
cursor. All five processes rejected that oldest token and reported exactly 1,024 retained records,
proving the hard eviction bound. The sequence allocated about 53.1 MB gross across 1,026 operations,
roughly 51.8 KiB per response-and-cursor operation, mostly repeated formation projection and JSON
work. Yet the process-wide live allocation delta was negative because unrelated earlier allocations
were reclaimed during the window. Reporting that negative value as “negative cursor memory” would
be nonsense. The defensible retained claim is the exact record count and eviction behavior, not a
byte estimate reverse-engineered from allocator motion.

Working set delivered a similar warning. Four idle pairs placed the on-minus-off delta between 8 KiB
and 274 KiB, while the first pair produced a negative 13.7 MB outlier. We retained that attempt. The
paired median was 143,360 bytes, but a five-pair local Windows working-set median cannot override the
tight allocation decomposition or identify a management owner. Startup page faults, runtime
warm-up, and allocator reuse can dominate a short process snapshot; this is exactly the sort of
result that should block an RSS claim rather than be edited away.

W6 therefore ends as measured-no-win. The request paths are bounded, disabled history is inert,
idle management has no periodic owner, cache reuse already removes most repeated aggregation work,
and cursors enforce both TTL and cardinality limits. No product mutation and no dedicated-host run
are justified for this surface. The useful output is a protected baseline and a preservation rule:
future changes must not introduce a background collector, bypass the aggregate cache, weaken the
cursor bound, or make disabled history contact an upstream service.

W3, the tag index, required a different decomposition. A tag participates in at least three kinds
of ownership: the immutable tag slice retained by a cache entry, the reverse index from a tag to
versioned keys, and the metadata copied into public cache events. Measuring only total process RSS
would merge those owners. We instead ran 215 independent release processes: 256 entries with
0/1/4/16/64 fixed-width tags, both a shared tag set and a unique tag set, 0/1/8 event subscribers,
and invalidation fan-outs of 1/64/1,024. Each cell had five repeats, exact memory reconciliation,
content checks for every delivered event, and its own stdout, stderr, and exit record. All attempts
completed, stderr stayed empty, and every invariant passed.

The exact retained-byte estimate grew by 184 bytes per logical membership and reached 3,014,656
bytes at 16,384 memberships. It was deliberately identical for shared and unique topologies. That
does not mean both layouts retain the same allocator memory: the estimator prices one logical
membership from known lengths and versioned constants; it does not inspect `HashMap` capacity or
deduplicate repeated strings. At 64 tags, the shared topology added about 154.8 live allocator bytes
per membership over its zero-tag cell, while the unique topology added about 413.9. The extra cost is
consistent with 16,384 distinct outer tag identities and their one-key maps. It identifies a real
topology cost, but not a safe optimization: global string interning would add synchronization,
reclamation, and adversarial-cardinality behavior to save memory only when names repeat.

Gross allocation told a related but separate story. Shared tags cost about 490.1 gross bytes per
membership and unique tags about 796.9. Some of that is unavoidable construction of entry and index
state; some is transient, such as forming an owned key for a `HashMap::entry` lookup. These numbers
are useful for later work, but changing index ownership first would mix several mechanisms and put
generation fencing at risk. The stale-load test therefore remained an explicit gate: invalidating a
tag while a load is in flight still discarded the stale store.

The event cells produced a much cleaner owner. After subtracting the corresponding zero-tag event
cost, one subscriber allocated 55.87 bytes per delivered 32-byte tag and eight subscribers allocated
56.00 bytes. The arithmetic explains the result: cloning one `String` needs its 24-byte header and a
new 32-byte payload allocation. `CacheEvent` currently stores `Vec<String>` and derives `Clone`, so
Tokio broadcast delivery repeats that deep copy for each receiver. With 64 tags and eight
subscribers, the shared-topology cell delivered 4,194,304 logical tag bytes and added 7,412,728 gross
allocation bytes relative to no subscribers; 7,340,064 of those bytes were the tag-dependent part.
The allocation owner scales with both tag count and subscriber fan-out while live retained memory
after delivery remains essentially unchanged. This is allocation churn, not a leak.

Invalidation did not expose another candidate. Every run removed exactly the requested 1, 64, or
1,024 entries, exact reconciliation ended with zero memberships, and one bounded generation record
remained to fence stale work. Median gross allocation at fan-out 1,024 was about 2.36 MB and elapsed
time about 1.23 ms locally, both consistent with required per-key removal. Optimizing that loop
without changing the removal contract would need a more specific owner than “linear in the work it
must perform.”

The W3 decision is therefore narrow: preserve the index and generation model, and preregister a
candidate that changes only event tag ownership from a deeply cloned vector to an immutable shared
slice. The public accessor can still return `&[String]`, event equality can remain content-based, and
`Clone` can become an atomic reference-count increment instead of duplicating every tag. Acceptance
must come from rerunning all 215 cells, not only the favorable 64-tag/eight-subscriber point; no-tag,
zero-subscriber, single-subscriber, shared/unique, invalidation, exact-memory, and stale-load controls
must stay green. These local figures locate the owner and authorize the experiment, but remain
non-promotable release evidence.

The preregistered experiment then changed one private field: `CacheEvent.tags` became an
`Arc<[String]>`. Constructors still accept the same inputs, `tags()` still returns `&[String]`, and
equality still compares contents. The important behavioral change is inside `Clone`: broadcast
receivers now increment a reference count instead of cloning the slice and allocating every string
payload. A focused unit test checks pointer identity after clone, while the existing public event,
slow-subscriber, tag-index model, exact-memory, and stale-load tests protect observable semantics.

The full candidate matrix again completed 215 of 215 attempts with empty stderr and no invariant
failure. In the primary shared-topology cell with 64 tags and eight subscribers, median gross
allocation fell from 15,628,164 to 8,686,476 bytes, a local reduction of 44.4% against a frozen 30%
acceptance threshold. More revealing than the total is the slope: tag-dependent churn fell from
56.00 to 3.01 bytes per delivered tag. With one subscriber it fell to exactly 24 bytes per tag. The
remaining 24 bytes are the `String` headers copied once while converting the constructed vector into
an immutable shared slice; with eight receivers that one-time cost amortizes to roughly three bytes
per delivery. The 32-byte string payload is no longer reallocated by each receiver.

The controls kept the conclusion narrow. Maximum no-subscriber gross regression was 1.11% against a
5% limit, maximum index live-delta regression was 1.41% against 10%, and invalidation gross allocation
did not regress. Logical membership counts, retained-byte estimates, fan-out removal, generation
records, exact reconciliation, event contents, and delivery counts were unchanged. The candidate is
therefore retained for integrated W3/W5 confirmation. It is a demonstrated local allocation win, not
a published latency or capacity claim; elapsed time remained diagnostic and no dedicated-host run
was spent on the isolated field change.

W4 applied the same method to the Redis compatibility path, but split the path into four stages before
looking for a change: wire decode, command translation, response encode, and complete roundtrip. The
frozen matrix covered both RESP dialects, ASCII and binary keys, key sizes from zero to 256 bytes,
batches from one to 256 arguments, distinct and duplicate topology, response payloads up to 1 MiB,
and six end-to-end controls. Ninety-two scenarios with five independent process repeats produced 460
attempts. Gross allocation was the ownership signal; elapsed time remained diagnostic because short
single-process timings on a developer workstation cannot support a throughput claim.

The first 460-process attempt was rejected, and retaining it exposed a useful profiling lesson. The
nominally distinct ASCII generator repeated after 26 bytes. Consequently, the 256-key DEL fixture
contained only 26 identities. Five processes exited successfully and wrote clean stderr, but both the
expected structured-byte invariant and command-cardinality invariant failed in every repeat: ten
false invariant fields across five attempts. A profiler that checked only exit codes would have turned
a fixture defect into a product conclusion. We fixed the generator to include a fixed-width unique
prefix, added a 256-key distinctness test, rebuilt the binary, and ran a second complete matrix. The
rejected raw manifest remains bound into the evidence alongside the accepted one.

The clean matrix confirmed that binary-key expansion itself is required compatibility work. A
non-empty key becomes `redis-binary-v1-` plus two lowercase hexadecimal characters per source byte,
so its structured length is exactly `16 + 2N`; the empty sentinel is 21 bytes. The measured GET cells
followed that identity at 16, 64, and 256 source bytes, and a distinct 256-key batch carried 36,864
logical structured bytes per operation. Those bytes cannot be optimized away without changing key
identity. Separating logical output from gross allocation prevents us from calling required data an
accidental copy.

Batch translation still exposed a secondary owner. At 256 binary keys, MGET and EXISTS allocated
about 55.5 KiB gross per operation, MSET about 43.3 KiB, while distinct DEL allocated about 116.8 KiB.
Duplicate DEL fell back to 55.8 KiB because it produced only one structured key. The additional
distinct-DEL work is consistent with follow-up ownership and linear deduplication, but changing it
first would touch order, duplicate semantics, and execution planning. It is now a measured follow-up,
not a license for an unfocused collection rewrite.

Decode showed an even larger but less isolated signal. RESP2 gross allocation ranged from roughly
2.05 to 6.19 times wire input in the frozen corpus. RESP3 ranged from 2.27 times for a large SET to
52.00 times for the small, array-heavy HC.TAG command; GET-64 was 14.40 times and the 256-key MGET and
DEL shapes were about 12.13 times. This deserves further ownership work, but the cost spans parser
frames and external value types. A broad parser change would combine lifetime, protocol, and API
risk, so those ratios are recorded rather than immediately “optimized.”

Encode provided the clean owner. A 1 MiB bulk response emitted 1,048,588 wire bytes but allocated
2,097,176 bytes gross per operation; array responses showed the same near-two-times relationship in
both dialects. The implementation fills a local `BytesMut`, then calls `to_vec()`, which allocates a
second buffer and copies the complete frame. In the pinned `bytes` implementation,
`Vec::from(BytesMut)` transfers a unique backing allocation and copies only when the buffer is shared.
The encoder's buffer is local and unique. That gives W4 a narrow candidate: replace the two final
copies with ownership transfer, keep every public response byte identical, and rerun all 460 cells.
The preregistered bar is at least 40% less gross allocation in every 1 MiB encode cell, zero wire or
roundtrip changes, and no more than 5% gross regression in decode or translation. Only that evidence,
not the attractiveness of the source diff, decides whether the change stays.

The implementation changed exactly the two registered return expressions. The RESP2 and RESP3
encoders now consume their `BytesMut` with `Vec::from(output)`. Compatibility tests still covered
golden scalar, array, binary, error, null, pipeline, boundary, listener, and mined-corpus behavior;
97 unit tests and 39 active integration tests passed, as did `clippy` and the full workspace check.
That test layer matters before measurement: a faster encoder with one changed byte is a protocol bug,
not an optimization.

The candidate matrix then completed 460 of 460 independent processes with empty stderr, zero failed
invariants, and no wire difference in any encode or roundtrip cell. All 70 pure decode and translation
medians were byte-for-byte unchanged in gross allocation. The four 1 MiB encode cells reduced gross
allocation by 49.96% to 50.00%, clearing the frozen 40% threshold. Bulk frames made the mechanism
especially visible: a 1,048,588-byte response went from 2,097,176 gross bytes per operation to exactly
1,048,588. At 4,096 payload bytes, bulk allocation likewise fell from 8,210 to the 4,105-byte frame.
The removed allocation is the full-frame copy, not a statistical inference.

Array encoding did not fall all the way to its wire length, which is the next useful distinction. At
1 MiB, RESP2 retained 640 gross bytes above its 1,048,741-byte wire frame; RESP3 retained 1,664. Those
small fixed residuals correspond to array-element frame construction and RESP3-specific structure,
not a second payload-sized buffer. They should not be folded into the same claim or chased without a
new size-scaling profile. Even the small encode controls improved rather than regressed, from 5.4%
for the empty RESP3 array to 50% for bulk payloads.

Local elapsed medians in the four large cells also fell by 32.7% to 41.4%, but those figures remain
diagnostic. Allocation identity is deterministic enough to accept the ownership change locally;
portable latency and capacity still require a qualified host and representative server workload.
W4 therefore keeps the two-line candidate for integration, defers RESP3 decode and distinct-DEL work
as separately measured owners, and does not spend a dedicated-host campaign on an isolated copy whose
semantic controls and allocation mechanism are already explicit.

W7 started with a terminology trap: durable memory is not one owner. It includes short-lived record
encoding and decoding buffers, Sled's own process state, allocator high-water pages, operating-system
file cache, logical bytes charged to the durable budget, and bytes written through the process. RSS
cannot separate those categories. The local profile therefore measured allocation phases, exact
logical bytes, directory length, process IO transfer, working set, and private commit independently.
On Linux it can additionally read anonymous and file PSS from `smaps_rollup`; on this Windows run that
split was explicitly unavailable in all 120 attempts. We recorded `null`, not an invented file-cache
number. Consequently, this run cannot support a page-cache residency claim.

The source audit found a more direct owner before any OS-memory interpretation was needed. Every
`DurableValueStore::upsert` asks `would_fit` whether the budget permits the record. `would_fit` reads
the existing record and calls `total_bytes()`. That method scans the complete Sled prefix and decodes
every durable record. Upsert then reads the existing record again, merges, encodes into a payload
vector and a second framed vector, inserts, and flushes. The sync coordinator flushes again; the
async coordinator queues cheaply, but drain calls the same flushing upsert for every item and then
performs one final flush. These are distinct candidate owners and must be measured separately.

The frozen matrix used 24 scenarios and five fresh processes each: store lifecycle at 1/16/64/256
records and 64/4,096-byte payloads; RAM-only, sync, and async-bounded write paths; and repair-pending
versus repair-confirmed tombstone GC. All 120 processes exited cleanly, reopened content matched,
logical-byte accounting reconciled, async lag returned to zero, GC cardinality was exact, and every
temporary store was removed.

Steady reads supplied the control. A 64-byte record cost roughly 167--185 gross bytes per read across
all cardinalities; a 4 KiB record cost roughly 4.20 KiB. Fill did not stay flat. For 64-byte payloads,
median gross allocation per upsert rose from 9.24 KiB at 16 records to 40.23 KiB at 256, a 4.35-times
increase. For 4 KiB payloads it rose from 67.84 KiB to 597.37 KiB, or 8.81 times. Overwrite grew even
faster: 7.43 and 10.02 times over the same cardinality interval. At 256 records, updating one 4 KiB
logical value allocated about 1.147 MB gross. Required record bytes are constant within each series;
the changing term is the full-store decode scan.

The durability modes showed where work moves. RAM-only admission stayed exactly flat at 114 gross
bytes for a 64-byte payload and 4,146 for a 4 KiB payload. Async admission performed zero write
transfer and cost only about 329--333 or 4,361--4,365 gross bytes while lag grew to the registered
64 or 256 entries. Drain then reproduced almost exactly the sync allocation and IO shape, and lag
returned to zero. That does not prove device-level write amplification: the Windows transfer counter
observes process IO above the storage stack. It does show that the current async queue defers the
full scan-and-flush work rather than batching it away.

GC preserved the safety boundary. With repair pending, it removed zero records and issued zero write
transfer. Once repair was confirmed, it removed and reclaimed exactly 64 or 256 tombstones. The
roughly 4.5 KiB gross allocation per removal and per-record writes identify the remove-and-flush loop,
but do not by themselves authorize weakening repair fencing or batching durability semantics.

The first W7 candidate is therefore narrower than “optimize Sled.” Cache only the logical byte total
used by admission, initialize it from one validated scan at open, and update it after successful
ownership changes. Keep the public validation scan, format and checksum behavior, budget rejection,
recovery, sync-before-ack, async backpressure, and repair-fenced GC unchanged. A candidate must flatten
the fill/overwrite cardinality slope in both payload series and rerun all 120 scenarios. File-cache
ownership and redundant flush work remain separately measured follow-ups rather than being bundled
into that counter change.

The implementation added a private `budget_used_bytes` value to the durable store. Opening a store
performs one checksum-validating scan to initialize it. Admission then combines that total with the
same existing-record and incoming-record byte arithmetic as before; a successful insert updates the
counter before flush, and remove subtracts only an actually removed record. The public
`total_bytes()` method still scans and validates every record, so the optimization does not turn a
cached admission value into an observability claim. A focused test exercises replacement shrinkage,
budget release after remove, missing-key remove, reopen reconstruction, and rejection at the restored
limit. Corruption, recovery, sync-before-ack, async backpressure, scrub, and repair-fenced GC suites
remained green.

The unchanged 120-process matrix passed again. At 256 records, fill gross allocation fell from
40.23 KiB to 7.78 KiB per 64-byte record, an 80.7% reduction, and from 597.37 KiB to 50.84 KiB per
4 KiB record, a 91.5% reduction. Overwrite fell from 70.23 KiB to 5.25 KiB and from 1.147 MB to
45.79 KiB, reductions of 92.5% and 96.0%. More important than any one endpoint, the 16-to-256
cardinality growth collapsed from 4.35--8.81 times to 1.11--1.43 for fill, and from 7.43--10.02
times to 1.01--1.10 for overwrite. The term proportional to all existing records is gone.

The tradeoff also appeared where preregistration predicted it. Combined reopen plus reopen-read
allocation increased by at most 4.68%, below the 10% ceiling, because reconstruction now validates
the store once. That one-time cost replaces a validation scan before every future write. Steady reads
and RAM-only admission did not regress, and maximum GC allocation regression was 0.114%. Exact logical
bytes, content after reopen, queue lag, GC removal, and temporary-directory cleanup all reconciled.

W7 therefore closes as accepted with one narrow product change. The local allocation percentages are
not release-grade performance claims, and the Windows run still says nothing about anonymous versus
file-backed residency. The per-record flush behavior remains a measured owner, but batching it would
change durability timing and was outside the one-candidate W7 authorization. Leaving that work
explicitly deferred is part of the optimization result: removing one proven owner does not grant
permission to redesign every adjacent subsystem.

W8 tested the allocator-high-water hypothesis without treating RSS as an allocator counter. The
existing 0.71 provider protocol had a subtle gap: setting `MIMALLOC_SHOW_STATS` proved only that a
provider was nominally enabled. Unless a separate metrics document was supplied, its snapshot path
fell back to process RSS and normalized that value into allocation-shaped fields. That fallback is
useful for old process-level diagnostics, but it cannot answer whether memory is live, committed,
reserved, resident, or reusable. The W8 contract therefore rejects RSS substitution and requires
every unavailable native field to carry a reason.

The profiling build selected exactly one of the existing `allocator-system`, `allocator-mimalloc`,
or `allocator-jemalloc` features. On Windows, system and mimalloc are applicable and jemalloc is an
explicit target-level non-applicability, not a failed candidate. The fixed trace used 16,384 entries
with 4 KiB payloads, 65,536 steady reads, exact full deletion, refill, and a two-second no-purge idle.
Five independent system/mimalloc pairs ran in alternating order. Five additional mimalloc processes
performed a force-collect checkpoint followed by a second refill. All 15 processes passed the exact
phase, cardinality, payload, stderr, and raw-evidence checks.

The native API audit itself produced a result. In the release mimalloc 3.3.2 build,
`mi_stats_get_json` exposed committed and reserved bytes, process information, arenas, faults and
purge counters. Its `malloc_requested.current` field remained zero because v3 provides no supported
process merge API in this binding; the older `mi_stats_merge` declaration has no linked v3 symbol,
and the main-heap JSON route returned no usable snapshot. We treated those probes as invalid and
recorded live/requested bytes as unavailable. The JSON's `process.rss_current` is likewise process
RSS, not allocator-owned resident memory, so resident remains unavailable and RSS stays in the
separate OS snapshot. Enabling mimalloc's debug mode would have changed the measured allocator build,
while copying RSS into either native field would have changed the meaning of the metric. Neither is
a valid repair.

The Windows comparison exposed a real tradeoff rather than a winner. Mimalloc's median trace time
was 4.13% lower, and refill incurred 3,158 new page faults versus 34,333 for system, a 90.80%
reduction. That is consistent with fast reuse of pages that remained committed. At the exact empty
checkpoint after deleting every entry, however, system working set was about 8.29 MiB and private
commit 4.92 MiB, while mimalloc remained at 148.92 MiB and 162.04 MiB: 17.95 and 32.97 times the
system values. After equal-cardinality refill and idle, mimalloc still used 8.75% more working set
and 11.71% more private commit. Faster reuse and lower idle footprint point in opposite directions;
choosing one number would hide the cost paid by the other.

Mimalloc also reserved roughly 1.076 GB throughout the run. That is virtual address space, not a
gigabyte of resident or live application data. It cannot be compared with the system allocator's
missing native retained field. This is why an allocator table must carry source and semantics beside
every byte count: identical units do not imply identical concepts.

The purge experiment did not resolve the tradeoff. Force collect advanced the native purge-call
counter by three and reported 1.125 MiB purged. Median working set fell only 0.55%; private commit
and native committed bytes rose by about 0.08%, and the second refill expanded working set to about
291.19 MiB. A positive purge counter proves that the API ran, not that the operating system recovered
useful capacity or that calling it in production is free. No purge policy was authorized.

W8 therefore retains the system default and records a terminal deferral for the Linux-only part of
the allocator matrix. This Windows screen is enough to reject a mimalloc default change, but not to
rank jemalloc or make a portable claim. A future allocator proposal must justify the expense of an
admitted Linux system/mimalloc/jemalloc run, preserve native missing-field semantics, and add an ADR
plus the complete compatibility, CPU, and latency matrix before changing any default. A measured
tradeoff is a valid optimization outcome: no product mutation is safer than selecting the faster
allocator while hiding its retention behavior.

W9 asked a deliberately different question: do the preceding measurements justify a new opt-in
limit on retained cache bytes? The existence of a retained-byte estimator does not answer it. An
estimator is a ruler; admission is a behavioral policy. The latter decides whether an operation is
accepted, which scope pays for it, how replacement deltas are reserved, how a failed batch rolls
back, what a retry observes, and when the reservation is released. Turning reporting into rejection
without a demonstrated pressure owner would be a semantic change disguised as instrumentation.

We preregistered two possible terminal outcomes before making that decision. `authorize-d2` required
one measured owner that was both attributable to logical retained bytes and still unbounded after
W2--W8. `not-applicable` was required when no such owner survived. RSS, private commit, allocator
arenas, virtual reservations, file-backed page cache, transient copied bytes, and an already bounded
queue were forbidden substitutes. These quantities matter, but a logical retained-byte limit cannot
promise to control them.

The owner-by-owner audit found no qualifying gap. W2's expiry work and W3/W4's tag and RESP changes
removed allocation or copy churn without discovering unbounded live retention. W6 kept already
bounded management services after a measured-no-win result. W7 already has a separate durable
logical-byte budget; conflating that on-disk owner with in-memory retained estimates would charge the
same application value for different lifecycles. W8's committed and reusable allocator pages are
external state: rejecting the next cache write cannot guarantee that an allocator purges old pages
or that the operating system lowers RSS.

W5 was the one real pressure finding, and it demonstrates why owner-specific admission comes first.
The HC/2 application queue now charges the encoded envelope against a one-MiB budget and holds the
permit until the item is polled or dropped. That closes the exact unbounded owner we measured. The
remaining producer frame, tonic/h2 flow-control window, protobuf/TLS buffers, socket state, and
allocator high-water do not share one logical retained-byte lifecycle. Adding a global cache limit
would double-limit the queue-adjacent request while leaving several of those external owners
untouched.

Existing controls also cover different, explicit contracts. The generic admission controller bounds
in-flight request bytes and FIFO depth. Multitenancy enforces request and value ceilings plus
tenant/namespace logical-value quotas. Its batch path prevalidates the final last-write-wins state,
commits accounting only after the mutation commits, and leaves the old usage intact on abort. These
properties are useful building blocks, but their existence is not evidence that another global
policy is needed.

We reran five local suites containing 32 focused tests. They covered estimator overflow and Moka's
`u32` boundary, exact replace/delete/flush reconciliation, capacity eviction accounting, request
permit release, retryable overload, tenant isolation, pre-mutation oversize rejection, duplicate-key
batch accounting, aborted-batch rollback, and idempotent quota release. The legacy builder still
weighs encoded value bytes for `max_capacity`; the reporting estimator's `try_moka_weight` adapter is
deliberately not installed. Thus no absent configuration silently changes behavior.

W9 closes as `not-applicable`, with no product or configuration change. This does not claim that
HydraCache can never need retained-byte admission. It defines the evidence needed to reopen the
question: equal-workload process or cgroup pressure must reconcile to a specific still-unbounded
logical owner after existing limits, and a new D2 contract must freeze scopes, reservation/release
semantics, rollback, retry behavior, absent-setting compatibility, and thresholds before candidate
code or measurements. Refusing an unevidenced feature is part of optimization discipline: every
limit consumes compatibility and operational complexity, even when its default is “off.”

W10 changes the unit of reasoning from isolated patches to one provisional integrated candidate.
Six product changes survived local screening: borrowed expiry keys, shared HC/2 event bytes, an
encoded-byte HC/2 outbound budget, shared event tags, RESP buffer ownership transfer, and the cached
durable budget total. Their percentages cannot be added. Three changes meet in the event-delivery
pipeline; expiry and tags share cleanup/accounting outcomes; RESP joins them in the mixed-protocol
workload; durability must remain a separate persistence companion. The integration ledger records
that composition order and those interaction groups before any combined measurement.

The first local qualification pass also caught two examples of a dangerous testing failure mode:
green or red commands that do not exercise the intended configuration. The initial durable command
exited successfully but ran zero tests because `durable_value_store` is feature-gated. We marked the
attempt invalid, added `--features durable-value-store` in a separate pre-rerun amendment, and then
executed all five durability tests. The original all-targets Clippy command failed for the inverse
reason: it tried to compile a durable compatibility example while the exports it imports were still
disabled. A second amendment kept the same packages and targets but enabled the required package
feature; the corrected warnings-denied run passed.

This is why exit status alone is not a test result. A gate needs a minimum executed-test count and a
declared feature/target matrix. Zero tests is not success, and a configuration error is not evidence
of a product regression. Both invalid attempts remain in the ledger instead of disappearing behind
their corrected reruns.

After correction, the integrated branch ran 342 tests with no failures. Another 25 tests remained
explicitly gated scheduled soaks, external Redis client/oracle checks, or resource smokes; we did not
silently count them as passes. The executed set covered client-surface compatibility and quota
release, event bytes/tags/order, HC/2 byte permits and real-mTLS drain, RESP2/RESP3 golden bytes,
memory and tag reconciliation, and durable reopen/corruption/budget behavior. All targets of the five
affected crates then passed Clippy with warnings denied, and 93 evidence-canary tests kept the
candidate, amendments, and non-promotion boundary immutable.

That local result opens only the next cheap step: an integrated process smoke for the four declared
interaction groups. It does not open a dedicated host, freeze final `C73`, prove compatibility with
the published 0.72 binaries, or justify six- and 24-hour runs. Expensive qualification begins only
after the combined process scenario can account for every outcome and can fail its own interaction
canaries locally.

The integrated smoke turned that boundary into executable code rather than another checklist. One
test binary is built once and then invoked as four independent processes: event delivery,
expiry/tag accounting, the frozen mixed-protocol workload, and a sync-acknowledged durable
companion. Every cell schedules exactly 1,000 operations and emits the same complete outcome vector.
The runner rejects a zero-test process, a missing receipt, a second receipt, any nonzero rejection,
timeout, late or incomplete count, or any change to the 35/30/15/10/5/5 mixed allocation. It keeps
stdout and stderr hashes per process and records the source tree and binary hash.

This local layer exercises real boundaries without pretending to be a benchmark host. The event
cell starts the production daemon and a real mTLS HC/2 stream. Three hundred concurrent puts are
allowed to fill the subscriber side before reads resume, so shared event tags and bytes meet the
encoded-byte queue budget under backpressure; the cell then proves monotone watermarks, zero drops,
and zero client and server owners after close. The mixed cell uses the same real daemon for 350 HC/2,
300 RESP and 150 HC/1 operations, adds 100 direct-cache, 50 tag-invalidation and 50 TTL operations,
and checks the management endpoint rather than trusting only client success.

The expiry/tag cell revealed why “local” should not mean “mocked.” Exact cache reconciliation proves
that entries, tag memberships and estimated retained bytes return to zero, but HydraCache's local
cache has no tenant quota. A real isolated `ClientSurfaceState` therefore fills a one-entry/value
quota, advances the active-expiry clock without reading the key, observes zero retained quota owners,
and refills successfully. The durable process similarly uses the feature-gated sled store rather
than an in-memory substitute: it covers overwrite, read, tombstone, repair-confirmed GC, reopen,
budget rejection and corrupt-envelope refusal while making no local timing or page-residency claim.

The failed attempts were as informative as the green run. A two-record durable GC scan repeatedly
visited the sorted live prefix and never reached later tombstones; increasing the number of calls
could not repair the wrong scan model. The first runner parser missed valid receipts because libtest
placed the JSON after its test-name prefix. The first stalled-event design assumed subscriptions
cross HC/2 streams, and a longer timeout merely confirmed that the semantic assumption was wrong.
All three failures remain in the evidence ledger with their corrections. This is the practical
difference between retaining failed evidence and silently retrying until green.

On the clean implementation commit, all four processes reported 4,000 attempted and 4,000 successful
operations in total, with zero rejected, timed out, late or incomplete outcomes. A fifth process
enabled `HYDRACACHE_CANARY_DEFECT=W10`, deliberately removed one success from accounting, and failed
with `HC-CANARY-RED:W10`. The canary matters more than the all-green summary: it proves that the
runner can reject incomplete work instead of merely recording it. This result admits design of a
focused protected-host comparison. It still supplies no throughput, latency, allocation, RSS,
capacity or release-improvement number and does not itself authorize an expensive dispatch.

The next step was not to start that expensive comparison immediately. We first made the host harness
prove that it could preserve the preregistered experiment. The standalone overlay compiles against
both exact product identities without editing either tree: frozen `I73` and provisional `C73` keep
their own server binaries, while byte-identical harness sources bind to each checkout through path
dependencies. The workflow verifies the candidate tree object, hashes both role binaries, both
harness binaries, the common overlay, the scenario, and the runner, then checks that neither product
worktree changed during the build. This turns “we probably tested the right revisions” into a
machine-rejectable identity condition.

Process placement needed the same treatment. The first harness measured the combined CPU of daemon
and load generator, but the daemon inherited the load generator's CPU affinity. A total CPU number
can still be arithmetically correct while the experiment violates its isolation policy. The corrected
harness accepts two distinct CPU sets, starts the daemon through its own `taskset` boundary, records
both sets in every receipt, and lets the runner reject a mismatch. This is a useful general lesson:
resource accounting and resource placement are separate assertions, and a benchmark needs both.

We also tightened event accounting between warm-up and measurement. Merely requiring at least as many
HC/2 events as successful puts allowed delayed warm-up events to mask a missing measured event. The
harness now waits for the exact warm-up event count, resets the event counter only after that drain,
and then requires exact equality for the measured 35% HC/2 share. At 1,000 local operations the
receipt therefore contains exactly 350 HC/2 events—not “350 or more.” This small change illustrates
why reconciliation should be phase-scoped: a correct lifetime total can hide a wrong measurement
window.

The campaign runner freezes the full expensive matrix instead of accepting convenient command-line
reductions: 5,000, 12,000, and 17,000 operations per second, ten-second windows, five pairs per rate,
and 5,000 warm-up operations. That is thirty independently started role processes. Order is derived
from the preregistered seed; missing roles invalidate a pair; every attempt retains stdout, stderr,
receipt hash, exit code, and actual command. For every rate it calculates within-pair C73-versus-I73
differences and only then applies the frozen Walsh-average Hodges-Lehmann estimator. Goodput, CPU per
completed operation, and p99 retain their 2%/3%/3% guards. RSS stays diagnostic, and combined-process
allocation is explicitly unavailable because adding an allocator counter to either frozen product
role would mutate the objects being compared. “Unavailable” is safer evidence than a precise-looking
partial allocation number.

Finally, the workflow is manual-only, protected by the admitted environment, and serialized with the
same host lease as the earlier campaigns. It captures calibration before and after the complete
block, refuses host-identity or lease drift, and uploads the packet even when a primary guard fails.
Before any real pair, it runs a defect-injection canary. The local canary removed one HC/2 success,
exited with `HC-CANARY-RED:W10-HOST`, and emitted no receipt. The normal local composition run
completed 1,000/1,000 mixed operations, all six exact surface shares, 350 measured events, owner
reconciliation, and the 1,000-operation durable companion. These local facts qualify the tooling and
open one manual host dispatch; they are deliberately not performance evidence and cannot finalize
`C73`, authorize a long run, or support a release claim.

The first protected-host attempts also exposed an important boundary: an expensive run starts only
when the first measured process starts. A dispatch rejected before a runner, a nested concurrency
deadlock, a missing collector CPU assignment, a Cargo overlay that cannot resolve its workspace, or
a stale worktree registration consumes engineering time, but none of them is a performance sample.
We retained every such attempt with its run and artifact identity instead of relabelling a later retry
as “the first run.” This keeps infrastructure selection out of the candidate result: a failed setup
cannot vote against the product, and a convenient retry cannot vote for it.

Persistent performance hosts make lifecycle state part of experiment design. Fixed temporary paths
look deterministic, but after cancellation Git may remember a worktree whose directory no longer
exists. Reusing that name then fails before compilation. The corrected workflow prunes missing
registrations and derives all product and overlay paths from `run_id` plus `run_attempt`; it also
writes an explicit incomplete manifest when canary or campaign artifacts do not exist. The broader
lesson is that fail-closed evidence must cover orchestration too: partial packets need a machine-
readable reason, and cleanup residue must never be mistaken for a product regression.

Once those orchestration defects were removed, the protected campaign finally crossed the boundary
into measurement. It completed all thirty independent processes: five counterbalanced `I73/C73`
pairs at 5,000, 12,000, and 17,000 operations per second. Across the matrix, 3.4 million offered
operations all completed successfully, with no errors, timeouts, or rejections; 1.19 million HC/2
events reconciled exactly, and all 30,000 durability-companion operations passed. The pre/post
calibration spreads were 1.33% and 2.26%, below the frozen 5% limit, with the same host identity,
policy, and lease on both sides.

The Hodges-Lehmann paired estimates passed every primary guard. Candidate CPU per completed
operation changed by +0.32%, +0.49%, and +0.12% across the three rates, against a +3% regression
budget. Goodput changes rounded to +0.005%, +0.0002%, and -0.004%, against a 2% budget. P99 changed
by -3.55%, -0.11%, and +0.14%, against a +3% budget. These are best read as “the integrated
optimizations did not create a material throughput, CPU, or tail-latency regression,” not as three
portable improvement claims. The low-rate p99 reduction is welcome, but five pairs on one admitted
profile do not make it a universal speedup.

Post-run RSS estimates ranged from -0.01% to +0.39%, and peak RSS from -0.01% to +0.33%. We retain
them as diagnostics only. The frozen product roles did not contain a combined-process allocation
counter, and patching one in after candidate freeze would have changed the compared objects. The
correct conclusion is therefore narrower: the focused integrated guard passed and opens the real
published-0.72 compatibility and rollback matrix. It does not promote RSS, establish allocator
improvement, finalize `C73`, or authorize six-hour and 24-hour runs before compatibility is proved.

The next local screen made that compatibility boundary executable rather than rhetorical. We built
one standalone harness twice: once against the exact commit referenced by the published `v0.72.0`
tag, and once against the frozen C73 commit. Those two client binaries were crossed with the two real
server binaries, producing four independent process cells. Every cell ran 18 assertions across
HC/1, HC/2, RESP, management routes, console assets, protocol-appropriate keys and binary values,
TTL expiry, tenant isolation, malformed input, tag invalidation where RESP exposes it, and live-owner
cleanup. All four
cells passed. The surface-applicability table matters: claiming an HC/2 tag test when HC/2 does not
offer that operation would be fake coverage, so the contract assigns each behavior only to the
surface that owns it and forbids substituting one surface for another.

Durable compatibility used the same discipline but, critically, not a fresh directory per phase.
The B72-linked binary created a live record and tombstone and reopened the store. The C73-linked
binary then read those exact old bytes, wrote a new record, flushed, and reopened. Finally the B72
binary reopened the same directory and read both the old record and the candidate-written record.
Separate fault stores proved repair-confirmed tombstone collection, rejection before a one-byte
budget could be exceeded, and loud checksum-corruption refusal followed by restoration from the
preserved raw record. This is stronger than serializing equivalent structs in two unit tests: the
actual old and new libraries took turns owning the same on-disk database.

We also falsified the orchestrator, not just the product. The canary deliberately removed the
`C73 client -> B72 server` cell. The matrix validator emitted the preregistered
`HC-CANARY-RED:W10-COMPAT` marker and produced no campaign pass receipt. The real local campaign then
recorded SHA-256 identities for both harnesses, both servers, every per-cell receipt, and every
durable transition. Two frozen lockfiles are retained because B72 and C73 resolve different product
dependency graphs; silently regenerating a single convenient lockfile would make the builds less
reproducible, not more.

This result is intentionally non-promotable. It rejects obvious client/server and durable-format
incompatibility cheaply on a developer machine, but it does not exercise leadership transfer,
mixed-version quorum behavior, follower restart, or same-disk daemon rollback. Those six rolling
scenarios remain the next gate, and long-duration performance runs remain closed until that gate is
complete. This is the purpose of a profiling ladder: spend seconds or minutes locally to eliminate
bad candidates and broken evidence logic, then reserve the expensive environment for the failure
modes that only a real cluster can reveal.

The local rolling driver then exercised exactly those six transitions. It bootstrapped three B72
processes so the initial leader was unambiguously old, upgraded both followers to C73, forced a
leadership change, restored a B72 follower on its existing storage, restarted that follower again,
completed the C73 rollout, and finally replaced one C73 process with B72 on the same disk. The third
local attempt passed all six states with a healthy quorum and readable old and new management views.

Why the third attempt? The first two failures were useful failures of the proof. The first demanded
`completeness=complete` after full upgrade even though the established management contract permits a
partial aggregate while quorum remains healthy. The second copied a 0.71-era expectation that the
management route disappears after rollback; B72 already contains that route, so an unauthenticated
probe correctly returned 401 instead of 404. Neither correction changed a product binary. We kept
both failed attempts, narrowed the assertions to actual B72/C73 guarantees, and reran. This is an
important distinction in profiling and release testing: weakening a product invariant to obtain
green is unacceptable, but correcting a version-inapplicable oracle is necessary. The audit trail
must make the difference visible.

The first retained Linux campaign then found a different weakness. All four crossed wire cells and
all three durable transitions passed, but the first rolling assertion failed: after the two follower
upgrades, a C73 follower had won an election instead of the original B72 bootstrap leader. That did
not demonstrate an incompatible message or disk format. It demonstrated that the test had confused
the leader observed after startup with a leader guaranteed to remain elected. Faster or differently
scheduled Linux process startup exposed the distinction that the local Windows sequence had hidden.
The packet was sealed as incomplete, with its successful wire and durable results still retained;
the missing rolling receipt prevented it from opening long runs.

The correction was topology control, not a product retry. Before upgrading any follower, the driver
now selects the B72 node with the lowest stable Raft election rank. If another B72 node happened to
win bootstrap, the driver stops that winner once, waits for the preferred B72 node to become leader,
restarts the stopped old node, and verifies that all three old nodes converge under the preferred
leader. Only then does it begin the six preregistered compatibility states. The receipt records
whether bootstrap already supplied that topology or the controlled precondition was needed. The
fourth local attempt passed with unchanged product binaries.

This is a broader profiling lesson: scheduling is part of the test fixture whenever a result depends
on role ownership. “Start the old binary first” is not a deterministic mixed-version topology, and
repeating until the desired leader appears would silently select a favorable attempt. Establish the
role through a declared deterministic rule, record the setup separately from the measured or
compatibility scenario, and let every subsequent transition fail without an automatic retry.

The next serialized Linux run passed. Its downloaded manifest binds tooling commit `9508330b`, the
published B72 commit, the frozen C73 commit and tree, and SHA-256 digests for the canary, wire/durable
campaign, and rolling receipt. The negative canary still removed one crossed wire cell and failed
without a pass receipt. The real packet contains four of four wire cells, three of three durable
transitions, and six of six rolling scenarios; the rolling receipt also says that the lowest-rank
B72 node was already the bootstrap leader in this attempt. We independently rehashed every nested
receipt after download instead of treating the green Actions badge as evidence.

Compatibility therefore opens the *contract* for the six-hour qualification; it does not justify
starting an improvised soak. The long run must first freeze equal I73/C73 duration, phase schedule,
offered work, resource checkpoints, complete outcome accounting, failure policy, artifact budget,
and the rule that decides whether a 24-hour confirmation may start. Nor does the compatibility pass
turn diagnostic RSS or allocation observations into performance claims. A gate should authorize
exactly one next decision, not erase the boundaries of every later gate.

For this release the frozen long-run shape is deliberately finite. I73 runs first and C73 second as
independent continuous processes, each for six measured hours at the already qualified 12,000
operations/second mixed workload; a later confirmation repeats the same identities for 24 hours per
role only if the six-hour pair passes. Both roles keep the 35/30/15/10/5/5 HC/2, RESP, HC/1, direct,
tag-invalidation and TTL/refill mix, 256-key cardinality, 4 KiB payload, production instrumentation,
CPU placement, and five-minute post-work idle. The six-hour pair is capped at 15 runner-hours and
the 24-hour pair at 58, so a configuration mistake cannot consume an unbounded lease.

One-minute checkpoints cover outcomes, surface counters, owner state, RSS, anonymous and file PSS,
CPU, faults, threads and file descriptors. I73's post-warmup Theil-Sen slopes and moving-block 95%
upper bounds are sealed before C73 starts; the candidate cannot redefine them after observation.
Both combined RSS and anonymous PSS must stay within those baseline-only bounds. Peak RSS and page
faults remain diagnostics, and the long pair retains the earlier 2% goodput, 3% CPU/op and 3% p99
regression ceilings. This single pair confirms endurance; it is not counted as five independent
samples and cannot manufacture a new improvement estimate.

The artifact policy is part of the experiment, too. Minute series and compact receipts are kept,
while values, credentials and raw payloads are not. A role may emit at most 64 MiB and the whole
packet at most 256 MiB. Missing or more-than-90-second-gapped checkpoints, a restarted process,
incomplete outcomes, nonzero final owners, host or lease drift, an oversized packet, a surviving
canary, or a favorable retry all fail the phase. This is how a long test remains an auditable
experiment instead of becoming twelve or forty-eight hours of terminal output.

We implemented and exercised that measurement path locally before spending the host lease. The
same integrated harness now has an exact long-run profile and writes append-only JSONL checkpoints
before work, once per interval, at the end of offered work, and after the idle/reconciliation phase.
On Linux each checkpoint samples the harness and daemon together: CPU time, RSS, peak RSS,
anonymous/file PSS, faults, threads and file descriptors. The runner validates sequence numbers,
time gaps, final owner reconciliation, exact operations and per-surface outcomes before it computes
any slope. I73's bounds are written before C73 can start, and the workflow statically contains one
I73 role, one C73 role and no confirmation command.

The local falsifier deliberately suppressed the final checkpoint. It completed the product work
but was still rejected because the receipt and checkpoint series were incomplete, and no campaign
receipt was admitted. A subsequent two-second I73-then-C73 screen produced five and four checkpoints
respectively and a compact 23,979-byte packet. Those counts are useful evidence about plumbing, not
about performance: Windows resource sampling was intentionally unavailable, the load was only 1,000
operations/second, and the phase explicitly sets both performance claims and confirmation to false.
The correct conclusion is that the expensive experiment is now runnable and falsifiable—not that
C73 is faster, smaller, or stable for six hours.

This cheap screen also paid for itself by finding an orchestration defect before host time was
booked. An input-verification refactor had left the identity-map return behind an earlier return, so
the first canary failed in the runner with `None` rather than reaching the harness. Moving validation
back onto the live path and adding a regression test fixed the class of error. Three earlier Windows
staging attempts likewise exposed package aliasing, symlink privilege, and missing root `docs`/lock
inputs. None was hidden by retrying the benchmark; each failure refined the reproducible build
fixture while product commits and release thresholds remained unchanged.

There is one more boundary before the six-hour pair. A new workflow file on a feature branch is not
automatically a registered GitHub `workflow_dispatch` endpoint. The local tooling receipt therefore
does not claim that host dispatch is open. It requires a reviewed adapter through an already
registered manual entry, preserving the protected environment and the shared serialized lease.
Only after that route is validated should the single 15-hour-capped qualification consume the
dedicated host.

The adapter is intentionally narrow. The already registered
`performance-host-admission-073.yml` remains the public manual entry and the sole owner of the
shared `performance-reference-073-host` concurrency group. A lease owner beginning with
`long-run-073@` selects exactly one reusable qualification job; the entry forwards the exact
tooling commit, lease owner and lease end, while the nested workflow uses a run-unique technical
group. This split avoids the nested-concurrency self-deadlock seen in an earlier campaign without
allowing two performance campaigns onto the host. The ordinary admission and published-0.72
compatibility modes remain separate branches of the same registered entry.

The protected environment approval is also part of the evidence chain, not an inconvenient click
to automate away. Dispatch first created a waiting deployment for the exact reviewed commit. The
same authenticated repository account approved that deployment, after GitHub confirmed it was an
allowed reviewer, and only then did the self-hosted job begin. The job subsequently verified the
tooling SHA, lease, required host tools and clean exact worktrees before compilation. This gives us
three distinct identities to audit later: who requested the run, who approved use of the protected
host, and which immutable bytes the runner checked out.

The first protected attempt is run `36278780653`, bound to tooling commit `8f8d4457`, with a lease
that covers the full qualification cap. Its setup, identity, tooling, exact I73/C73 materialization
and four release builds passed. The injected missing-final-checkpoint canary was rejected, then the
pre-I73 host calibration passed, so the first six-hour I73 process started. That is still not a
partial performance result: it proves only that the registered path reached the admitted host and
that the measurement pipeline rejected its known defect before accepting real work. I73 completion,
its post-calibration, C73 pre-calibration, the equal-duration C73 role, its post-calibration and final
packet sealing must all pass in this same attempt. No replacement run is created merely because a
later stage might fail.

The attempt eventually completed both six-hour roles rather than failing early. Each role executed
259.2 million operations, produced 362 checkpoints, drained its backlog, reported zero errors,
timeouts, and rejections, reconciled all observed events and management owners, and passed the
durable reopen/corruption checks. Independent verification of the downloaded packet matched all
twelve embedded receipt, checkpoint, stdout/stderr, and calibration SHA-256 values. Goodput was
effectively unchanged, CPU per operation improved by 0.20%, and p99 increased by 1.43%; all three
stayed inside their frozen regression budgets.

The final qualification still failed, correctly and narrowly. C73's moving-block 95% upper slope
bound was 3.7926 bytes/s, above I73's already sealed 3.2508 bytes/s bound, for both combined RSS and
anonymous PSS. The raw Theil-Sen point estimate moved in the favorable direction (34.13 to 9.84
bytes/s), but the contract explicitly makes the upper-bound comparison decisive; a favorable point
estimate cannot rescue it. This is why a job may perform twelve hours of healthy work and then exit
red at sealing: successful execution is not the same thing as passing a preregistered decision.

The initial audit found no tooling or orchestration fault. Changing the estimator after seeing
these values, rounding away the 0.54 bytes/s gap, loosening the bound, or selecting a retry would
have converted a qualification into post-hoc threshold fitting. Run `36278780653` was therefore
retained as a complete failed attempt, automatic retry remained forbidden, and the 24-hour
confirmation was not opened.

A later consistency check found a different problem without changing the statistical question. In
both roles the reported “upper 95% bound” was below its own Theil-Sen point estimate: 3.25 versus
34.13 bytes/s for I73 and 3.79 versus 9.84 bytes/s for C73. That is not evidence that the candidate
barely lost. It is evidence that the implementation and the label described different statistics.
The analyzer sampled contiguous blocks of absolute memory *levels*, randomly concatenated those
levels, and then assigned the selected blocks new increasing time coordinates. Randomizing levels
this way erased the original trend before calculating each bootstrap slope. The resulting
distribution was centered near zero regardless of the trend the point estimator had just measured.

The repair was derived from the already-used `memory-statistics-071-v1` contract rather than from
the desired verdict. It converts adjacent checkpoints into rates in their original time direction,
resamples contiguous twelve-rate blocks with the same seed and 10,000 iterations, restores the
original number of deltas, and takes the 95th percentile of the resampled mean rates. A new
falsifier uses a perfectly linear 36-checkpoint series: both the known slope and its resampled upper
bound must remain exactly 1/6 unit per second. The old code failed this property; the corrected code
passes it deterministically.

We then downloaded the original artifact again and reanalyzed it offline. The verifier matched the
original campaign digest plus all role receipt, checkpoint, stdout, stderr, and calibration hashes.
No product process was rerun, and no workload, source identity, threshold, block size, iteration
count, or seed changed. The corrected I73 upper bound is 117.0801 bytes/s and the corrected C73
bound is 62.7351 bytes/s for both combined RSS and anonymous PSS. C73 therefore passes the frozen
candidate-at-or-below-baseline rule; goodput, CPU/op, p99, correctness, durability, and
reconciliation were already green.

This correction does not delete the first verdict. The original red campaign and analyzer output
remain immutable evidence of a tooling defect, while the new receipt records an append-only offline
reanalysis. It also does not automatically open confirmation or declare the later registry-packaged
candidate final: D4 must still review the analyzer correction and the `hydra-moka` distribution
identity. The immediate operational consequence is nevertheless useful—there is no justification
for renting the host merely to repeat the same six-hour observations in search of a green sample.

That distribution review produced a useful counterexample to the tempting phrase “the code is the
same.” The Git revision used by the campaign and `hydra-moka 0.12.15-hydra.1` have the same 52
runtime source files. A canonical path-and-content manifest has the same SHA-256 on both sides, and
the release `Cargo.toml` is byte-identical to the registry package's `Cargo.toml.orig`. The only
release-commit changes outside those sources are CI version pins, package metadata, changelog, and
README. HydraCache enables the `future` feature, while the only README include is guarded by the
`sync` doctest configuration. Runtime sources do not read the Cargo package name or version.

Those facts establish source equivalence, not build identity. Cargo now resolves a different
package name, version, source, checksum, and lockfile entry. Those values participate in the build
graph, and we did not establish byte-identical final HydraCache executables. More importantly, the
rule was frozen before seeing either result: a dependency or source change after candidate freeze
invalidates affected receipts. Relaxing that rule because inspection suggests the change is benign
would make identity enforcement optional exactly when it becomes inconvenient.

The old twelve-hour observation is therefore retained but not transferred to the registry-backed
candidate. It proved the workload, exposed and then helped falsify the analyzer defect, and passed
all guards under the corrected method for the old identity. It does not admit the new identity to
confirmation. The cost-saving move is to finish versioning, release notes, generated/package
assets, features, SBOM inputs, and lockfiles before renting the host again. Only then should the
final candidate receive one six-hour qualification and, if green, one 24-hour confirmation. This
avoids both dishonest evidence reuse and a second invalidation caused by late release packaging.

The first dispatch of that final registry candidate exposed one more orchestration boundary before
any performance sample was taken. The standalone harness overlay was byte-identical for I73 and
C73, but its single `Cargo.lock` encoded I73's local packages as 0.72.0 and Moka as the frozen Git
revision. I73 compiled; C73 presented the same path packages as 0.73.0 and `hydra-moka` as a registry
package, so Cargo refused to rewrite the lock under `--locked`. That red build is useful evidence:
both product servers and the baseline harness were buildable, while the candidate harness was never
created, the canary never started, and no timing or memory observation exists. Treating it as a
performance loss—or silently rerunning it—would be a category error.

The correction keeps the comparison symmetric without pretending that different product identities
have the same dependency graph. The overlay now carries two reviewed lockfiles: the original I73
lock and a C73 lock derived for the frozen registry candidate. The workflow verifies both SHA-256
values, builds each harness with its matching lock under `--locked`, restores the canonical overlay
tree, and performs a recursive equality check before the canary or either role can run. Thus the
harness source and workload remain byte-identical, dependency resolution remains reproducible, and
the temporary role-specific build input cannot leak into the measured overlay identity. The failed
artifact and its two post-calibration files remain append-only evidence; the frozen product commits,
workload, durations, estimator and thresholds were not changed.

This incident generalizes beyond Rust. “Use the same harness” and “use one lockfile” are not the
same requirement when the harness deliberately links two released product graphs. Reproducibility
means pinning each graph explicitly and proving that all non-product inputs return to the same state
before measurement. A locked failure during setup is cheaper and more trustworthy than allowing a
package manager to resolve dependencies online during a rented-host campaign.

The corrected build then reached the expensive part and exposed a second, subtler identity defect.
I73 completed its full six-hour role—259.2 million successful operations, 362 checkpoints, exact
reconciliation and no errors, timeouts or rejections. All surrounding calibrations remained on the
same admitted host and lease. C73 nevertheless stopped in one second, before starting its server:
the Python runner supplied the final frozen candidate SHA, while the Rust harness still contained
the earlier source candidate in its own allow-list. The harness correctly rejected the mismatch;
the orchestration had failed to update and cross-check two copies of the same identity.

That failure revealed an even more important flaw in the negative canary. The canary was meant to
prove that a packet missing its final reconciled checkpoint is rejected. Its implementation treated
*any* exception as success, including a harness startup error with no receipt and no checkpoints.
In other words, the guard was red, but for the wrong reason. A negative test is trustworthy only
when it proves both halves of the claim: the producer successfully reaches the intended fault, and
the consumer rejects that exact fault. “The command failed” is not sufficient evidence.

The repaired canary is therefore narrow and fail-closed. It first requires a zero process exit and
both output files. It validates the C73 identity and every ordinary receipt field after neutralizing
only the declared final-checkpoint flag. It then proves that the checkpoint stream ends in
`final-work`, contains no `post-idle-reconciled` record, and is rejected by the normal validator. A
startup failure, absent packet, unrelated malformed field or unexpectedly accepted packet now makes
the canary fail. Structural Rust and Python tests also bind the harness allow-list to the final C73
SHA so that changing the runner alone cannot recreate this split identity.

The six hours of valid I73 data remain useful diagnostics, but they are not half of a result that we
may splice into another run. The preregistered experiment requires serial I73 and C73 roles plus a
single final seal under one attempt. Because C73 never started and the campaign was not sealed, the
attempt is retained as incomplete evidence, automatic retry remains off, and confirmation stays
closed. This is painful on a rented machine, but it preserves the distinction between saved compute
and valid comparative evidence.

This ordering keeps the meaning of green steps monotonic. A successful build says the intended
bytes are executable; a successful canary says a known-invalid packet is rejected; a successful
calibration says the host is admissible at that boundary. None of those statements predicts the
next one, and none can be combined into an early claim about C73. Long-running evidence becomes
valid only when the final sealer can point backward to every required boundary receipt.

The corrected campaign finally demonstrated that property end to end. Run `36532416869` first
proved its negative control for the intended reason: the producer exited successfully, emitted an
otherwise valid C73 packet whose stream ended at `final-work`, and the ordinary validator rejected
the missing `post-idle-reconciled` checkpoint. It then held one admitted host and lease across four
calibrations and two serial six-hour roles. I73 and the final registry-backed C73 each completed
259.2 million operations, emitted 362 checkpoints, drained the backlog and reconciled events and
management ownership exactly, with no errors, timeouts or rejections.

Downloading a green artifact was not the end of the audit. We hashed the ZIP independently and
matched GitHub's recorded digest, recomputed every nested receipt, checkpoint, stdout/stderr and
calibration SHA-256, then reran role validation and the frozen comparison logic against the raw
packet. That matters because the previous campaigns had shown three different ways for a green or
red surface signal to be misleading: a build could fail before measurement, an identity mismatch
could stop only the candidate, and an overbroad negative canary could pass on an unrelated startup
failure. The independent pass reproduced the sealed statistics and all five guards rather than
trusting the workflow conclusion alone.

The result is deliberately modest and precise. Goodput changed by -0.000004%, effectively zero.
CPU seconds per completed operation increased by 0.316%, and p99 increased by 0.274%; both are far
inside their frozen 3% budgets. C73's RSS and anonymous-PSS Theil-Sen slope was 21.0045 bytes/s
against I73's 30.5605 bytes/s. More importantly, the preregistered moving-block 95% upper bound was
61.7818 versus 86.3800 bytes/s, 28.48% lower, so the boundedness decision does not depend on a
favorable point estimate alone. This is stronger than saying “no obvious leak”: the candidate
survived equal-duration baseline comparison under the exact estimator that previously exposed and
corrected our bootstrap mistake.

It is also not permission to overstate the result. A single integrated six-hour pair does not prove
portable capacity, universal latency improvement, or final release behavior. The small positive
CPU and latency deltas are accepted regressions within budget, not improvements. The qualification
opens a separately authorized 24-hour confirmation for the exact frozen candidate; it does not
substitute for that confirmation. We therefore recorded `confirmation_allowed = true` alongside
`confirmation_started = false`, kept `final_c73_allowed = false`, and did not launch another paid
run automatically. The useful engineering lesson is that cost control and evidentiary rigor are the
same workflow: cheap falsifiers eliminate broken orchestration early, while an expensive success is
accepted only after its raw packet survives an independent audit.

The separately authorized confirmation then exposed an orchestration lifetime risk rather than a
product failure. Run `36622527013` completed the full 24-hour I73 role: 1.0368 billion successful
operations, 1,442 checkpoints, exact reconciliation, no errors, timeouts or rejections, and a
frozen RSS/anonymous-PSS upper slope bound of 32.5177 bytes/s. C73 started on the same admitted host
and lease and remained healthy for 352 checkpoints. At 5 hours 51 minutes it had completed
252,728,137 operations with no errors, timeouts, rejections or major faults. GitHub Actions then
delivered `The operation was canceled`; the harness had emitted a heartbeat one second earlier,
stderr was empty, and the immediate post-C73 host calibration passed. The retained Actions log does
not name the cancellation initiator, so the narrow supported conclusion is an external orchestration
cancellation—not a C73 regression and not a successful confirmation.

The incomplete artifact was still valuable because the workflow's `always()` path preserved all
four calibrations, the complete I73 receipt and bounds, the partial C73 series, process logs and the
negative canary. We matched the downloaded ZIP to GitHub's SHA-256 and independently hashed every
retained nested file. We did not estimate a final C73 slope from the prefix, splice the completed
I73 role into a later run, or silently restart the candidate. Equal duration and a single final seal
remain part of the experiment, so `final_c73_allowed` stays false.

The tooling correction reduces the blast radius without changing the experiment. Qualification
continues as one serial six-hour pair. Confirmation is split into two sequential, role-isolated
jobs under one approved parent workflow and one host concurrency lease. The first job runs I73 and
uploads a continuation packet whose files are all SHA-256 listed. The second job verifies those
hashes and the complete I73 receipt before it can start C73, repeats binary and host identity checks,
and alone may seal the pair. Each job stays below 29 hours, while product commits, role order,
24-hour duration, offered load, estimator, seed and thresholds are unchanged. This is not checkpoint
resume: a failed role still invalidates the attempt, and a replacement confirmation must start from
I73 after explicit authorization.

The replacement showed why a savepoint must be a cryptographic boundary, not merely “some files we
can reuse.” Run `36839197349` completed I73 in its own job, sealed 13 files in
`continuation-SHA256SUMS`, and uploaded the packet. The C73 job downloaded it independently,
verified every hash, required the complete 1.0368-billion-operation I73 receipt and exact final
reconciliation, then repeated the build, host, lease and calibration guards before starting C73.
Only that second job was allowed to seal the combined campaign. Losing the GitHub job boundary no
longer meant losing a valid completed role, but changing a role, reusing a partial role or combining
different attempts still failed closed.

Both 24-hour roles then completed: 1.0368 billion operations and 1,442 checkpoints each, zero
errors, timeouts and rejections, exact surface/event/durable accounting, and the same admitted host
and lease. Independent analysis matched the two provider ZIP digests, every nested continuation
hash, both stdout/stderr hashes, four calibrations, raw checkpoint-derived statistics and all five
guards. Goodput changed by -0.0000000115%, effectively zero. CPU per operation increased by 0.260%
and p99 decreased by 0.328%, both comfortably inside the frozen budgets. The RSS and anonymous-PSS
95% upper slope bound fell from 30.1448 to 20.3181 bytes/s, a 32.60% reduction. The candidate
therefore passed the preregistered boundedness decision; the long pair still does not become a
portable claim about capacity, allocations or another product.

Archiving was treated as another correctness boundary. Actions artifacts expire, so all eight ZIPs
from the campaign history—including rejected and interrupted attempts—were copied byte-for-byte to
the append-only `evidence/0.73/w10-confirmation` branch. A commit-pinned manifest, full-file
`SHA256SUMS`, extracted accepted packet and verification receipt make the release decision
replayable after provider retention ends. We also disabled Git text conversion inside the archive:
without that small detail, a Windows checkout could change line endings and make truthful hashes
appear corrupt. Finally, the verifier hashes runner and scenario bytes from the frozen Git commit,
not the platform-transformed worktree. Reproducible evidence depends on preserving byte identity at
both boundaries.

## A green long run can still fail release admission

The final exact-tree CI check provided one last useful falsification. PR #214 pointed at the measured
candidate `16d2e98b6cc9e22d9ccf95eb26fe28bbbcf80f2b`. GitHub tested a synthetic merge commit, but its
tree hash exactly matched the candidate tree, so the result cannot be dismissed as merge drift. The
HC/2 workflow passed its Linux, Java 17, Java 21 and Docker jobs. The main CI and documentation
workflows did not pass.

The failures were mundane but release-relevant. The docs examples lockfile still named the local
HydraCache crates as 0.72.0 after the workspace moved to 0.73.0, so a locked documentation build
correctly refused to rewrite it. The frozen topology inventory did not list the newly added 0.73
workflows. The memory ownership registry did not yet include the removal observer and HC/2 outbound
queue. The gated-test registry omitted the rolling-compatibility environment gate, a retained 0.72
test required the whole workspace to remain exactly version 0.72.0, and the instruction tripwire
ended without benchmark summaries. Several admission jobs then failed by design because they
aggregate those upstream results.

None of those outcomes invalidates the 24-hour boundedness measurement, and none can be repaired by
reinterpreting its statistics. They invalidate publication of that exact Git identity under the
current release contract. A lockfile or registry can be easy to edit, but a frozen candidate cannot
be edited in place. The honest next step is therefore a reviewed choice: freeze a corrected
candidate and repeat the evidence whose identity changes, or explicitly revise the release contract
without converting a red gate into a waiver. Until then the failed CI attempt remains evidence,
`v0.73.0` remains absent, and the successful long run remains necessary but insufficient.

The project chose the corrected-candidate path. The correction is deliberately narrower than the
measured product: runtime sources, workload, duration, estimator, seed and thresholds are frozen;
only publication inputs, governance registries, compatibility assertions and CI lock preparation
may change. The 0.72 docs lock is regenerated for 0.73, and the instruction tripwire now rebuilds
an independent harness lock for each side before reconciling an intentional dependency transition.
The rejected run remains append-only. All affected exact-SHA gates must run again, while the
six-hour and 24-hour product packets remain attributable because no measured runtime path changed.

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

## Release 0.74: when eliminating copies was still not enough

The first 0.74 investigation applied the same discipline to the RESP pipeline. Source inspection
showed two persuasive costs: `Vec::drain` moved the unread suffix after every decoded command, and
every reply performed its own write and flush. Neither observation was accepted as an optimization
result until a dedicated local tool counted commands, consumed and moved bytes, response frames,
writes, flushes, allocations, CPU, latency, lock time and retained state under an identical trace.

The attribution was unambiguous. A pipeline-10 GET moved 265.5 suffix bytes per command; SET moved
1453.5. Each command also produced one high-level write and one explicit flush. Pipeline-1 moved no
suffix bytes, giving the input candidate a natural unaffected control. Separate native receipts
also prevented a RESP improvement from being confused with raw embedded, typed-codec, or direct
client-surface cost.

The first cursor implementation eliminated repeated suffix copies, but it deferred clearing a fully
consumed buffer until the next read. Counterbalanced processes exposed a small pipeline-1 penalty.
An eager-clear revision removed that avoidable transition. It achieved the intended mechanical
result—zero moved suffix bytes for complete pipeline-10 batches—but still failed the frozen local
non-regression guards: the five-pair median showed -2.45% SET pipeline-1 goodput, just beyond the
-2% limit, and +4.30% GET pipeline-1 CPU/op, beyond the +3% limit.

Those figures are local rejection evidence, not portable performance claims. The important result
is the decision. We retained every sample, did not widen the thresholds, documented the noisy pairs,
and reverted only the input-buffer candidate. The next independent hypothesis—bounded output
coalescing—remained eligible because its owner, tests and rollback boundary were distinct.

This is a useful refinement of “optimization starts with measurement”: making the suspected cost
disappear is necessary, but not sufficient. The candidate must also survive its unaffected controls.

The response-side experiment repeated the lesson in a different form. Bounded connection-local
coalescing reduced high-level writes and explicit flushes by exactly 90% for pipeline-10. SET
throughput improved by a repeat-backed local median of 28.27%, but GET improved by only 3.06%
against the frozen 20% floor. SET pipeline-1 also showed a 5.56% p99 regression against the 3%
general guard. The mechanical counter was green while the product outcome was not.

That distinction matters because a syscall-shaped hypothesis can be compelling even when the
chosen local transport makes syscalls unavailable. The in-process duplex fixture proved ordering,
batch boundaries, partial-write handling and high-level call reduction; it did not prove a kernel
write reduction on the real TCP server. The correct decision was again to retain the raw receipts,
label the measurements non-promotable, and revert the isolated candidate. A later direct-encoding
proposal must earn its own allocation result before any reconsideration of response batching; the
two hypotheses cannot be credited as one benchmark candidate.

The next attribution pass demonstrated the value of estimating an owner's ceiling before changing
code. A dedicated stage tool measured gross allocator requests for decode, translation-context
creation, command construction plus translation, and response construction plus encoding. The GET
encoder owned 264 bytes per operation, only 6.67% of the matched end-to-end allocation; SET encoding
owned 8 bytes, or 0.19%. Translation owned about 10% in both cells. None could reach the frozen 20%
floor even if eliminated perfectly.

So no direct-encoder or request-context candidate was written. Combining several independently
named W4 hypotheses to manufacture a threshold-sized result would destroy attribution. The useful
outcome was narrower: close those candidates as measured below the isolated floor and move the
profiler to key ownership, which crosses parser, translation and store boundaries and therefore may
have a large enough single owner to justify its migration risk.

The key pass then supplied a second kind of negative result. A 28-byte binary-safe RESP key expands
to a 72-byte canonical segment, and `stable_key()` allocates those 72 bytes again for lookup. That
157% representation expansion looks dramatic in isolation, but the lookup materialization is only
about 1.8% of end-to-end allocation. Even charging the entire 399-byte translation stage to the key
would reach only about 10%.

Because key representation affects compatibility, durable identity and rollback, the decision was
recorded as an ADR rather than hidden in a benchmark note. HydraCache 0.74 retains the existing
canonical identity and defers binary variants, interning and hash handles. A visually large local
amplification is not enough to justify migration when its share of the product cost remains below
the registered floor.

The next native profile found a materially different shape. At one client, live GET waited about
35 nanoseconds for the client-surface store mutex; at eight clients, aggregate wait rose to about
14.8 microseconds per operation while the measured critical section was about 1.0 microsecond.
Separate TTL fixtures classified every expired request as a direct cleanup rather than allowing a
mostly-miss workload to masquerade as expiry work.

This evidence authorizes a narrow read/read concurrency experiment, not immediate sharding. Live
hits may be tested under shared ownership while expiry and every mutation retain the canonical
exclusive path. Tenant admission, quota release and replacement races remain semantic constraints;
the cheaper proposal has to fail before a more invasive shard table or expiry index is considered.

It did fail. A shared read lock improved the single-client GET cell by 16.45%, but eight-client GET
lost 3.74% goodput and used more than three times the baseline CPU per operation. Expired GET at
eight clients lost 31.62%, as the fallback paid both read and write ownership. PUT and raw embedded
controls also crossed local guards. The candidate was therefore rejected despite its attractive
single-client result, and the failure was not reinterpreted as permission to jump directly to store
sharding.

Multi-key attribution then prevented work on code that was already vectorized. Batch-eight MGET,
MSET and EXISTS each crossed the client surface with one dispatch and one store acquisition. DEL
was different: one BatchGet plus eight individual invalidations produced nine dispatches and nine
locks. Only that DEL follow-up earned a candidate. The distinction avoids claiming a generic
"multi-key optimization" and gives the next patch a precise rollback boundary.

The DEL experiment is a particularly sharp example of why an unaffected-surface guard must be a
release condition rather than an explanatory footnote. A native batch-invalidation seam reduced
the eight-key operation from nine dispatches and locks to one. Five local pairs showed roughly 48%
more pipeline-1 goodput and 49% more pipeline-10 goodput, with materially lower CPU, allocation and
p99 in both cells. By the primary metric, the hypothesis worked.

The built candidate still failed. Longer native controls—used after a short screen exposed Windows
CPU quantization—showed ClientSurface PUT at eight clients losing 3.95% goodput and typed embedded
GET at eight clients losing 7.10%. Their p99 distributions also crossed the frozen guard. Those
paths did not call the new batch seam, but that is not grounds to discard the measurements: users
run binaries, not source-level arguments about which function should be unaffected.

So the receipts were retained and the candidate was reverted. The lesson is stronger than “check
for cost shifting.” A focused optimization can win its own benchmark by a wide margin and still be
the wrong release composition when independent controls reject the exact built artifact.

## The measurement process is part of the evidence

Release 0.74 also exposed a less obvious source of false confidence: a long-running benchmark can
have a sound estimator and still lose its identity when the CI controller disappears. Restarting a
harness from a checkpoint is not reattachment. The allocator, RSS history, process-local caches,
socket state and timing sequence belong to the original process lifetime; a replacement process
creates a new attempt.

The local supervisor work therefore treats launch as a durable transaction. PREPARED and STARTING
states contain no placeholder PID or invented checkpoint. Before any process side effect, the
supervisor persists a spawn intent that binds the campaign, request, manifest, nonce, role and a
deterministic systemd unit name. After this boundary a retry may only inspect that exact unit. If it
is absent, the attempt fails incomplete; if its full process identity matches, recovery adopts it;
if identities conflict or multiple executors appear, the campaign is quarantined. It is never
silently respawned.

This requires two levels of serialization. A process-held host flock prevents concurrent mutation,
while a persistent active-campaign marker prevents another campaign from taking the same admitted
host after a supervisor restart. Per-campaign state still uses revision-checked replacement. The
event journal records PREPARED, STARTING and the spawn outcome before advancing each snapshot, so a
crash after the journal sync but before `state.json` replacement has one deterministic repair:
finish that exact compare-and-swap from the authoritative hash-chain.

The live C74 rehearsal exposed the important final edge of that rule: quarantine also needs a
reviewed terminal cleanup path, otherwise a correctly preserved failure can reserve the host
forever. The implemented path is intentionally asymmetric. It can release only an exact
`IDENTITY` spawn mismatch with no retained process identities, no duplicate executor, and a
failed/inactive empty deterministic unit. A signed abort first becomes durable, diagnostics bind
the request, spawn evidence and unit snapshot, systemd resets the failed transient unit, and only
the completed `ABORTED_INCOMPLETE` transition releases the host claim. The rehearsal also showed
why retry authorization must be separate from the durable intent: the first backend attempt can
outlive the packet's authorization window. A fresh signed request now records a second
`AbortRequested` at the exact current revision before continuing; it cannot change the original
campaign, mismatch cause, or unit identity.

Fake-backend tests are valuable here because they can stop at every unsafe boundary: after intent
but before launch, after launch but before result, after a lost backend response, and after an event
append but before snapshot replacement. They proved the local exactly-once decision logic without
starting HydraCache or touching a real systemd unit. The next slice then implemented, but did not
execute, the production boundary. A live authenticated `start` now imports and revalidates the
immutable evidence, acquires the host claim, advances the lifecycle and dispatches through a
systemd backend. The backend builds one fixed `StartTransientUnit` request: exact argv with no
shell, a digest-bound minimal environment, the frozen CPU mask, runtime/memory/FD/task bounds and
unprivileged ownership. It admits only the original MainPID and one daemon when boot id, process
group, cgroup path/inode and cpuset all remain exact; a third process is a duplicate executor.

The same durable coordinator now starts C74 only from the exact sealed I74 revision. The second
signed `start` request has no staging path, reuses the already imported immutable evidence, writes
independent `c74-spawn-*` intent/result files and follows the same observe-only recovery rule after
the intent boundary. Unsealed and stale requests have no backend effect; a lost backend response is
adopted without a second start; an accepted response is replayed byte-for-byte from the journal.

That distinction is still essential. At source `286863e4`, 114 ordinary supervisor tests pass
under WSL, while the only real-system-bus inspection remains explicitly ignored. This is confirmed
local implementation evidence, not confirmation that a provisioned Linux host preserves either
role through controller and supervisor loss. Installing the service, creating measured units,
restarting the controller/supervisor, measuring overhead and exercising lease/abort/seal behavior
must still pass before this mechanism can support a release claim.

The broader lesson is that reproducibility includes process continuity. A hash-identical workload
executed by a replacement process is not the same sample, just as a benchmark with a changed seed
or duration is not the same sample. Long-run infrastructure must be able to retain an incomplete
attempt instead of manufacturing continuity.

The first real-host supervisor overhead screen added another useful rule: unavailable is not zero.
For 30 seconds the stable installed process consumed 0.00755% CPU and peaked at about 4.54 MB RSS,
comfortably below the already frozen idle ceilings. Its PID, start ticks, cgroup and CPU mask did
not change; there were no restarts, product requests or host mutations. Those are confirmed
observations, not estimates.

But the service cgroup did not expose `io.stat`, and the unprivileged runner could not read the
root process's `/proc/<pid>/io`. Treating that as zero bytes per second would have converted a
telemetry gap into a green budget. The receipt instead marks I/O unevaluated, keeps the complete
idle screen false, and leaves role-level overhead qualification open. The failed preflights were
retained because they identified two host-interface facts—the missing I/O controller and the need
to read affinity from `Cpus_allowed_list`—without changing the service.

This is the same discipline applied to infrastructure rather than product code. A partial result
can narrow the next experiment, but only the full frozen conjunction can close the gate. When the
measurement channel is missing, the next task is to establish that channel under the same trust
model, not to relax the claim.

The next isolated change established that channel instead of weakening the test. The host already
advertised the cgroup-v2 `io` controller, so the signed service and fixed transient-unit policies
enabled systemd I/O accounting without expanding the sudo command or accepting workflow-selected
properties. After a controlled signed reinstall, the runner could read the service's `io.stat`.

The repeated 30-second observation then passed the complete supervisor-idle conjunction: about
0.00662% CPU, 4.42 MB maximum RSS and zero cgroup I/O bytes per second, with the same process and
cpuset throughout. This does not turn one idle window into a role-overhead claim. Checkpoint I/O,
I74/C74 timing and asymmetric paired controls are different measurements and remain open. What
changed is narrower and important: a missing measurement channel was converted into an exact,
least-privilege counter, and only the gate actually covered by that counter was closed.

## A durable writer still needs a consistent reader

The first full signed start/attach/abort rehearsal exposed a failure that no mocked backend had
made visible. The checkpoint writer followed the intended durable order: append one canonical
JSONL record, fdatasync it, write a new head file, then atomically replace the old head. Maintenance
happened to read after the journal sync but before the head replacement. It saw the new record and
the previous head and classified the pair as corrupt.

That classification was wrong. Each file was valid; the observer had sampled between the two
commits. Its fatal exit restarted the root supervisor, whose private systemd mount namespace then
had a different identity. The recovery path correctly refused to stop an ambiguously identified
unit, but the original trigger was an avoidable reader race. The failed run was retained with all
12 checkpoints, 58 supervisor restarts and the final fail-closed diagnostic instead of being
discarded as infrastructure noise.

The repair did not weaken journal verification. The reader now samples the head, verifies the
journal, samples the head again, and accepts only an unchanged head equal to the verified journal
tip with no incomplete tail. A head transition or partial appended record receives a bounded
100-millisecond retry; stable malformed input and permanent hash-chain errors still fail closed.
Two deterministic tests hold the files in each real commit window before completing the writer.

The same rehearsal found an independent containment mismatch: the daemon was intentionally
bounded to 900 seconds, but its manifest produced only 360 seconds of systemd `RuntimeMaxSec`.
The corrected non-product bundle uses six 145-second phase budgets plus 30 seconds of diagnostic
grace. The lease and every product duration remain unchanged.

After exact-source reprovisioning, one GitHub run signed start, attach and abort for the same
controller principal. Attach accepted checkpoint sequence 6; the writer reached sequence 11;
abort stopped the exact unit and released the host claim. The supervisor kept one PID with zero
restarts, and checkpoint I/O caused no NVMe interrupts on measurement CPUs 1-4. This is confirmed
non-product lifecycle evidence, not a throughput or release claim. Live seal, supervisor restart,
reboot, role overhead and the expensive product qualification still require their own proofs.

The lesson generalizes beyond this supervisor: crash consistency is not enough when one logical
state spans multiple durable files. Readers must define which inter-file snapshots are valid,
which are transient, and how long they may retry without turning real corruption into availability.

## Durable identity cannot depend on a namespace-local number

The next real-host experiment restarted the supervisor while the exact non-product fixture pair
continued to run. The initial hypothesis was simple: the persisted process, cgroup, cpuset and
mount receipt should let the fresh supervisor adopt the existing pair without respawning it. The
process part worked. The fixture retained its PIDs and start ticks, the campaign state file did not
change and no duplicate appeared. One identity field did change: the numeric mount ID moved from
324 to 430 because the restarted service entered a new private mount namespace.

Treating those numbers as a durable equality key would reject a valid restart. Ignoring the mount
altogether would be worse because a changed device, root, path, filesystem, source or option set
could then pass. The admitted projection is deliberately narrower: the full receipt still records
the numeric mount ID for diagnosis, while the stable digest excludes only that namespace-local
field. Tests mutate every semantic mount component independently and require each mutation to
remain identity drift.

The negative rehearsal found two more boundary conditions. A signed attach transfers the active
controller lease to its GitHub run principal, so an abort dispatched as another run was correctly
rejected. Recovery orchestration therefore performs attach and abort in one run rather than
weakening lease ownership. Later, the 900-second fixture exited successfully before abort signing
finished. Systemd retained the exact unit as `active/exited` with `MainPID=0` but released its
cgroup. Requiring the old non-empty cgroup in that terminal shape caused a fail-closed diagnostic
loop. The verifier now accepts an empty cgroup only for that exact successful terminal unit; live
units and non-empty foreign cgroups still fail.

After those isolated fixes, the repeated rehearsal passed at source `9178f3ab`. Restarting only the
supervisor changed its PID from 65781 to 74207 while fixture PIDs 73941/73942, their start ticks and
the campaign-state digest stayed constant. The fresh process adopted the pair, signed attach
accepted checkpoint sequence 7, and a same-run signed abort ended at revision 5
`ABORTED_INCOMPLETE`. Thirteen checkpoint records remained verifiable, no duplicate executor was
observed, and the unit, processes and active host claim were absent at the end. The runner was
returned offline and the supervisor remained active with zero restarts.

That is confirmed lifecycle evidence, not a performance result. No product candidate ran; no
throughput, latency, role-overhead or isolated-CPU interrupt claim was measured. Live seal,
live-role reboot and expensive product qualification remain separate gates. The useful result is
architectural: a restartable controller needs semantic identities that survive namespace renewal,
and each relaxation must be no wider than the lifecycle state that justifies it.

## A complete control-plane rehearsal is still not a product benchmark

The next admitted-host run completed the path that the earlier abort and recovery experiments had
left open. A freshly provisioned exact source produced a new root-bound host observation and one
immutable six-file start bundle. I74 then crossed signed start, deterministic terminal wait,
terminal-evidence attach and seal. C74 was admitted only from that exact `I74_SEALED` revision and
crossed the same sequence to `COMPLETE_SEALED`.

This mattered because final seal owns more than a state label. It must verify the retained
successful unit, clear every process/checkpoint/controller-lease field and release the host-wide
claim only after the durable event chain and final state agree. At the end, both role units were
`active/exited` with zero MainPID and `Result=success`; the campaign had no failure, corruption or
duplicate-executor flag; the active marker was absent; the supervisor had zero restarts; and the
Actions runner was offline again. Every signed request and response, the start bundle and the final
event head are content-addressed in the retained receipt.

The important measurement lesson is negative: completing infrastructure does not create a
throughput claim. The roles were bounded lifecycle fixtures, not HydraCache product candidates.
They measured authorization, identity continuity, exact revision transitions and cleanup. They did
not measure goodput, latency, allocations, RSS under product load or native-API non-regression.
Accordingly, live seal and typed mutation coverage can turn green while product qualification,
role-overhead, live-role reboot and release admission remain red. Keeping those statements
separate prevents a reliable benchmark controller from being mistaken for a fast cache.

## A reboot must invalidate a live measurement

The next experiment tested the remaining reboot boundary with a live non-product role. The fixture
was admitted through the same signed start path, reached `I74_RUNNING`, published a durable
checkpoint chain and held the only active-host marker. The runner was then stopped and the machine
was deliberately rebooted. The old process pair could not survive, but absence alone was not enough:
the restarted supervisor still had to explain the loss, make it durable and avoid replacing the
workload.

That is exactly what happened. The boot ID changed; the enabled supervisor recovered the marker and
classified the retained process identity as `host-identity-drift`. It first recorded a cause-bound
measurement-loss intent and diagnostic, then cleared the old harness, daemon, checkpoint and lease
identities, committed `FAILED_INCOMPLETE` and released the host claim. The role unit was absent, no
new fixture was spawned, the runner stayed disabled and the supervisor needed no restart.

The useful idea here is that infrastructure recovery and measurement recovery are different things.
The supervisor should recover after a reboot, but a performance sample tied to the previous boot
must not. Turning this gate green therefore means proving deterministic failure and cleanup, not
making a broken run appear continuous. It remains non-product evidence: role overhead, HydraCache
throughput and latency, the six-hour qualification, the 24-hour confirmation and release admission
still require their own measurements.

The next overhead step therefore starts with the admission shape, not a convenient number. The
analyzer requires five counterbalanced control/instrumented pairs for each role, preserves I74 and
C74 separately, and binds every attempt to the same workload, payload, seed, operation count,
warm-up, cpuset and host receipt. It checks the frozen supervisor CPU/RSS and control-plane I/O
ceilings on every instrumented attempt, while elapsed and CPU overhead asymmetry are compared across
roles. A control cell that emits checkpoint bytes, or any identity/placement/operation drift,
invalidates the set.

Just as importantly, the staged analyzer cannot promote its own result. Synthetic or non-product
attempts may prove that the measurement logic is sound, but they always leave product overhead and
release admission false.

The first fixed-host executor tested the wrong trust boundary. Commit `57b3e56d` added a bounded
no-argument role-overhead verb to the root-owned provisioning entrypoint, but signed provisioning
run `37461600977` rejected it before changing the host: the bundle installer no longer matched the
installed bootstrap command. This negative result is useful. A signed payload must not be able to
grow the sudo command that installs it, even when the proposed verb itself looks narrow. The host
was restored with the original supervisor active, zero restarts and the runner offline.

The corrected design keeps that entrypoint byte-for-byte unchanged. An unprivileged collector now
builds the frozen 20-cell ABBA schedule and launches only `taskset`, `nice` and a hidden bounded mode
of the installed supervisor binary, with no shell and fixed resource limits. It independently binds
the source, binary and provisioning receipt, samples supervisor cgroup-v2 CPU/I/O plus process RSS,
and rejects boot or PID/start-time, exact argv, cgroup and cpuset drift. The root observation
separately binds the live binary digest. Control emits no checkpoint; instrumented execution
writes and fsyncs exactly 4,096 bytes.

One subtle guard mattered more than the launch mechanism. The Actions user cannot traverse the
root/service-group campaign directory, so `test ! -e active-campaign` can succeed for both absence
and permission denial. Treating that as an idle host would manufacture evidence. Each attempt is
therefore bracketed by the existing typed host-observation request. The root supervisor checks the
marker before and after collecting its immutable observation and returns
`active_campaign_absent=true` inside the response digest. Thus the collector gains proof, not new
privilege, and any active or malformed claim fails closed.

The corrected collector still produces rehearsal-only evidence. Before its admitted-host run, the
completion flag stayed false, and even a passing result could not replace exact I74/C74 product
pairs. Those product pairs remain a separate evidence boundary.

The first unprivileged run found one more host-specific boundary before measuring anything. Linux
ptrace policy correctly prevented the Actions user from dereferencing `/proc/<root-pid>/exe`, so run
`37465569900` stopped before attempt one. The fix did not request more privilege. It replaced that
single inaccessible link with an exact readable tuple: command line (binary path, `serve`, fixed
config), unified cgroup, PID/start ticks and cpuset. The bracketed root observation still binds the
live binary SHA-256. This is a useful pattern for instrumentation: when a strong-looking check is
unavailable in the real security context, compose independent readable identities with an existing
trusted observation instead of weakening the host or silently skipping the guard.

The following signed run found a second measurement-interface edge before attempt one. A fresh
`IOAccounting=yes` cgroup can expose a readable zero-length `io.stat`: the kernel has no block
device row to report until that cgroup actually performs block I/O. Rejecting the empty file made a
valid zero look like missing telemetry. The narrow correction treats a readable empty file as zero
but still rejects a missing or unreadable file, malformed device rows and rows without byte
counters. This distinction matters: accepting the kernel's canonical zero is not the same as
turning absent evidence into a passing metric.

With that distinction fixed, run `37467960862` completed the exact 20-cell schedule. The measured
fixture cost was consistent across roles: median elapsed overhead was 1.271387% for I74 and
1.291742% for C74, while median process-CPU overhead was 0.043175% and 0.045724%. The elapsed and
CPU asymmetries were 0.020355 and 0.002548 percentage points, far below the frozen one-point
ceiling. Supervisor CPU and block-I/O deltas were zero in every short pair, peak supervisor RSS was
5.00 MiB, and the maximum combined checkpoint/control-plane rate was about 8.83 KiB/s against a
1 MiB/s ceiling. All identity, placement, workload and campaign guards passed.

Those numbers validate the measurement path, not HydraCache performance. The executable was a
bounded synthetic fixture, and the instrumented cells intentionally performed one 4 KiB fsync.
Therefore the run closes only the non-product rehearsal flag; product role overhead, the six-hour
qualification, the 24-hour confirmation and release admission remain open. The useful result is
that later product pairs can reuse a host-proven collector without granting more privilege or
quietly converting unavailable counters into zeros.

## Write the release boundary before there is a release candidate

The next useful step was not another benchmark. W2 through W9 had produced terminal negative or
deferred decisions, W10 had zero candidates to compose, and W11 had proved substantial controller
behavior without running a HydraCache product pair. At that point an optimistic release note could
easily turn infrastructure progress into a performance story that the evidence did not support.

W12 now encodes the opposite. Its pre-admission contract fixes C74 as unresolved, disables product
and expensive runs, forbids numerical product, portable-capacity and Redis-superiority claims, and
enumerates every missing qualification, supply-chain and archive receipt. The qualification
manifest hashes this contract, so enabling a run or relaxing a claim boundary also invalidates the
prepared manifest. A release-scoped expected-red canary flips the expensive-run flag and proves
that the checker rejects the mutation.

The draft release note is useful precisely because it is not a launch announcement. It tells the
reader that no product optimization has been accepted, preserves 0.73 as the product rollback
baseline, separates opt-in W11 tooling from the runtime, and records each failed or deferred
hypothesis. It also states what would invalidate future evidence and what must be repeated on an
exact frozen C74.

This is a general optimization lesson: negative results need release engineering too. A failed
candidate is safely finished only when its code is removed, its evidence is retained, its claim is
excluded, and the final admission machinery cannot accidentally treat local diagnostics as shipped
performance. The local W12 preparation is now complete on that narrow definition. Product
qualification, packaging and the immutable archive remain deliberately red.

The final local reconciliation also exposed a useful vocabulary trap. Once all W2-W9 negative,
deferred and not-authorized outcomes were connected to retained artifacts, the release-evidence
tool reported all 28 work-item contracts as `Implemented`. It still reported zero fast-green,
gated-green and ship-ready items. That distinction is intentional: implementation of an evidence
contract means the result can no longer disappear into prose; it says nothing about acceptance of
the candidate that the evidence rejected. Treating “recorded” as “shipped” would undo the whole
measurement discipline at the final gate.

The next boundary applies the same rule to tests. All 28 work items are now attached to the existing
workspace-wide nextest gate without changing its command, timeout or budget. A smaller contract test
would be faster, but letting it stand in for the workspace suite would create a false green edge.
Registration therefore improves traceability while the stage remains `Implemented`: only an
exact-commit receipt from the unchanged full gate may advance it to `FastGreen`.

Refreshing the W11 canary on Windows then demonstrated why its green guard runs first. The canary
never executed: one Linux-only fixture function was referenced by an unguarded CLI match arm, so
the supervisor binary failed to compile with E0425. The fix was one platform guard, matching all
adjacent host-only entrypoints and leaving Linux semantics unchanged. The failed guard remains
negative evidence; a compile failure cannot masquerade as an expected-red canary.

The first full workspace gate then supplied another negative result. The evidence runner deliberately
used an isolated Cargo target, and that cold Windows build was still compiling when the unchanged
2,400-second hard timeout expired. It had already missed the 840-second cadence budget; no test had
started and no JUnit report existed. Calling this a test failure would be as misleading as calling it
green. The durable record therefore says `compile-phase timeout`, keeps both limits unchanged, and
forbids promotion. A single same-command warm-cache retry can attribute the failure to cold-build
capacity, but it cannot overwrite the cold result or establish release admission by itself.

The warm-cache retry made the attribution sharper without making the gate green. Compilation
finished in 11 minutes 24 seconds, after which nextest scheduled 3,655 tests across 477 binaries.
The complete command still reached the hard timeout, returned only after process termination, and
never produced JUnit. No failure had been reported, but an interrupted suite cannot prove that no
failure exists. The useful conclusion is narrow: this local machine cannot complete the unchanged
workspace gate inside its current cadence and hard-timeout contract. Repeating the same run again
would add heat, not information; the next attempt belongs on an appropriately provisioned ordinary
CI lane or after a concrete test-runtime root cause is identified.

## A deadline must cover the evidence collector too

The long timeout receipt suggested a smaller question worth answering locally: can output capture
outlive the deadline even when the registered process has already exited? The runner already used
`taskkill /T /F`, so attributing the delay to killing only the parent would have been premature.
Inspection instead found unbounded waits for that utility, the child and the pipe-reader threads.

A short fixture made the failure reproducible. Its parent exited successfully, but a descendant
kept stdout/stderr open for four seconds. With a one-second gate limit, the runner waited 4,070 ms
and wrote `Pass`. The problem was larger than a slow timeout: a successful parent could make the
receipt green while its unfinished descendant and output capture escaped the command deadline.
The fixture's expected-timeout test failed before the implementation changed, and that negative
baseline remains checked in.

The Windows repair follows the ownership model provided by
[Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects). The command
starts suspended, joins a private kill-on-close job, and only then resumes its initial thread. This
ordering closes the race where a newly launched parent could spawn a child before containment.
The runner checks both the parent's status and the job's active-process count while the output
readers drain their streams. All three must finish inside the command deadline before `Pass` is
possible. On timeout, the job owns termination even if the original parent has already exited;
cleanup polling has an explicit two-second bound. Containment and capture errors reject evidence.

The clean-source regression run passed all 15 runner tests. With a one-second limit, the three
fixtures returned `Timeout` in 1,071 ms for inherited pipes, 1,057 ms for a descendant with null
streams, and 1,106 ms for a live three-generation tree. Four seconds later, none had written its
survivor marker. Another fixture verified complete capture of 128 KiB on each stream and a Unicode
tail. A local Linux compile check and all 12 applicable runner tests also passed, using their own
toolchain and target directory.

These are diagnostic observations about the test runner. They establish a reproduced false-green
boundary and its local repair; they do not identify the precise cause of the earlier 226-second
workspace termination overrun. The full workspace gate still needs a completed receipt on the
frozen source. Preserving that distinction keeps a useful tooling fix from becoming an unsupported
claim about product throughput or release readiness.

The final CI audit found an ownership gap to close before the next ordinary hosted run. Selecting
`candidate_release=0.74` already labeled the full workspace receipt correctly, but the fast lane
did not execute the release's two canaries or its contract checker. The existing lane now does so
conditionally before the same workspace command, then builds a non-ship release report from the
receipts and uploads it alongside JUnit. This makes one ordinary dispatch reviewable as a coherent
source-bound test run while keeping product qualification behind its separate authorization.

That audit also exposed a quieter coverage defect: twelve already implemented Linux-only supervisor
test targets were not registered, and eight existing 0.74 workflows were missing from the global CI
inventory. Adding their exact sources, commands, platforms, dependencies and ownership makes them
visible to the same governance checks as older releases. It does not add a new product hypothesis or
turn isolated fixtures into full-workspace proof. The inventory retains the established publication
producer and 0.73 records. Artifact names bound to signed request UUIDs or bundle content digests
remain intact, with explicit expiring exceptions explaining the identity rather than bypassing it.

Three sensitive steps also lacked a separate deadline. Their new bounds limit a profiler build and
two host-identity reads without changing the measurement workload or acceptance thresholds. Bundle
assembly now has its own source/host/run concurrency group: reusing the child observation workflow's
host group would risk serializing a parent against its own child. The touched attribution and host
capability jobs require explicit dispatch, preventing this preparatory push from starting host work.

Local evidence supports the wiring: 33 performance-contract tests passed, including a new parsed-YAML
guard for ordinary CI defaults and explicit host dispatch; topology and all 17 governance checks
passed. Local WSL ran 206 supervisor tests successfully while leaving one opt-in real user-bus test
ignored. These are deterministic fixture and metadata checks, not throughput results. Windows hit
its command-line limit for a single workspace formatting invocation. The equivalent check then
passed package-by-package for all 36 workspace members, without editing unrelated files. Recording
both the limitation and the successful alternative keeps the result reproducible. The next proof is still the
unchanged full workspace gate on an ordinary hosted CI run, not an expensive qualification campaign.

The first ordinary dispatch confirmed the infrastructure boundary: the two touched host workflows
were triggered by the push, but all their jobs were skipped. It then exposed two metadata failures
that the narrower local preflight had not covered. The draft release notes lacked the site's exact
include wrapper and navigation entry. Separately, the conservative ownership scanner interpreted
`BTreeMap` inside `MutexGuard` generics as a candidate owner and correctly rejected the missing review.

Neither failure justified weakening a gate. The documentation repair includes the canonical notes
and labels them as a development draft, without implying publication. Source review showed that
`ProfiledStoreGuard` stores a borrowed mutex guard, a borrowed instrumentation reference and inline
timing metadata; `ClientSurfaceState` still owns the map. A precise registry entry records that fact
and avoids counting the map twice. A new fixture verifies that normal drop and panic unwind release
the lock with profiling on or off, while preserving mutex poisoning. The scanner, registry closure,
five ownership tests and strict transport lint then passed locally, as did release-doc synchronization
and mdBook. These repairs change no product behavior and supply no allocation-reduction claim.

The original hosted run continues to own its original source and failures. Passing local checks on
the repaired source cannot be combined with that run to manufacture a green release receipt. Its
timestamped in-progress evidence is retained separately; the repaired source still needs its own
completed ordinary CI proof before any stronger readiness claim.

Dependency policy found one more omission in that same run: both internal 0.74 workspace tools
lacked package license metadata. The downstream Docker job consequently rejected the upstream
Rust result before running interop tests; this was not an independent wire-protocol failure. A local
license-only check and a new inheritance regression reproduced the missing metadata before the
repair. Both tool manifests now inherit the workspace's existing `Apache-2.0` declaration. The
license checker and all 34 performance-contract tests passed with the allowlist, confidence threshold,
exceptions and lockfile unchanged. Restoring missing metadata is materially different from relaxing
a supply-chain gate, and neither substitutes for the still-pending exact-source CI proof.

### Frozen measurement inputs are not live registries

The first ordinary CI run eventually completed the entire workspace command. Including compilation,
it took 607,210 ms on that hosted runner, with 3,754 of 3,756 executed tests passing and two failing.
JUnit was captured and there was no timeout. That is useful operational evidence: one hosted run
fits the unchanged 840-second cadence that the local Windows attempts could not meet. It says
nothing about HydraCache goodput, allocations or native API performance, and two failed tests still
make the receipt fail. The final negative result is retained separately from the earlier in-progress
observation, with source, artifact and JUnit digests.

One failing test was the missing guard review already diagnosed. The other caught a mistake in our
own CI inventory work: the topology JSON under `memory/0.71/` was included in a frozen baseline's
scenario inputs. Extending it silently changed a measurement prerequisite. Adding the guard review
to the neighboring frozen owner registry would create the same problem. Recomputing the baseline
hashes would hide the defect, not repair it.

The repair therefore restores both historical snapshots exactly and separates ongoing bookkeeping
from historical evidence. The topology checker reads a current repository-wide catalog outside the
frozen directory. Ownership validation combines the frozen registry with a mandatory additions file,
then runs the same production scanner and review validation. Additions must preserve the registry's
schema, release and discovery scope. They cannot replace an existing symbol review or duplicate
another addition. Stale reviews, expired exemptions, unresolved test references and newly unreviewed
owners remain failures. The guard's borrowed map disposition is unchanged; it merely lives in the
appropriate extension file.

We first added a digest regression and observed it fail, then implemented the separation. The new
guard verifies all twelve frozen input digests. On clean repair source, eight baseline tests, five
ownership tests, 35 performance-contract tests, seven governance tests and two merge fixtures passed.
Linux also passed the merge fixtures as a development diagnostic. W11 and W12 mutations both
produced their required expected-red failures. Formatting, strict library lint and live inventory
checks passed without weakening thresholds or changing product behavior.

The lesson is practical: a file named "registry" can still be an immutable measurement input. Its
role follows its consumers, not its filename. Live review data needs an explicit extension boundary
that preserves historical identity and fails closed. These results repair verification machinery;
they neither validate a product candidate nor turn the original failed CI into a green run. The next
step remains a new ordinary CI execution on one exact repaired source, with costly qualification
disabled.

### An ordinary green report must not open ship admission

The audit while ordinary CI was running exposed another W12 boundary defect. The dedicated 0.74
performance-contract command rejected ship admission, but the generic release-evidence aggregator
did not share that decision. Once ordinary receipts were green, empty gated-proof lists could let
its generic progression label a row `ShipReady`. That label would overstate what had actually been
checked: no candidate identity or expensive qualification had been completed.

We reproduced the missing admission reason with a new regression before implementation. The
aggregator now shares the same closed boundary as the performance-contract CLI and retains it in
the report's reasons. Both `0.74` and `0.74.0` reject `--require-ship`. Ordinary-green work can reach
`FastGreen`, but not gated qualification or ship readiness. A separate unit fixture supplies the
counterfactual "all checks green" booleans and verifies the ceiling, alongside the unchanged
progression for older releases. The fixture is deliberately synthetic; it is not release evidence.

On clean repair source, 109 library tests, 36 performance-contract tests and 11 aggregator tests
passed. Both CLI spellings rejected admission, and W11/W12 canaries produced expected red. Linux
also passed the stage fixture. Earlier Linux registry diagnostics needed scoped Git metadata paths
because the Windows worktree's `.git` file was not directly interpretable by Linux Git. After those
environment overrides, the baseline and ownership suites passed without changing the checkout.
The broader Windows test-target lint still fails on a pre-existing non-Linux supervisor dead-code
warning; the changed-library strict lint passed. Recording that distinction avoids calling an
incomplete platform check green.

The ongoing hosted run belongs to its earlier source and does not contain this additional repair.
Its ordinary test outcomes can be retained, but any ship-ready labels from that older aggregator
must be rejected as release authorization. A fresh exact-source ordinary CI is still needed for
the repaired tooling. This correction improves the honesty of readiness reporting; it adds no
product throughput or allocation claim and starts no qualification workload.

### Passing tests, meeting cadence and authorizing a release are different proofs

The second ordinary hosted run completed on its original repaired-registry source. All 3,761
executed workspace tests passed; 85 tests were skipped. The old ownership, documentation and MSRV
failures disappeared on that source. However, strict Linux Clippy failed in the supervisor tool,
so downstream HC/2 admission refused to proceed. This was an upstream tooling rejection, not a
new interop failure. Nonfatal cache symlink warnings were not the cause.

We reproduced the lint failure locally using CI's pinned Rust 1.94.0 rather than relying on the
Windows-only preflight: 24 production diagnostics and four additional test-target diagnostics.
The baseline supervisor suite passed 206 tests with one opt-in systemd check ignored. We added an
observation regression before changing representation: even when no process pair exists yet, a
foreign systemd unit must remain a terminal mismatch, not become a retryable startup observation.
The repair then passed 207 tests, leaving the opt-in real-service check disabled.

Most changes are straightforward propagation and scoping repairs: return the response builder's
`Result` directly, remove an unnecessary slice borrow and final optional reborrow, and compile a
Linux-only helper on Linux and in tests. The two private sum types are expressed with standard
inline `ControlFlow` and `Option`, preserving owned guards, terminal-versus-pending behavior and
drop paths. We deliberately add no indirection or heap ownership. The lint repair therefore does
not trade a warning for a new `Box` allocation. It also does not demonstrate a smaller stack
representation or reduced product allocations; those would require their own measured hypothesis.
The rehearsal writer receives the same owned values in a context structure, with its channels and
timing unchanged. Moving the test module and keeping constant timing relationships as runtime
checks completes the test-target cleanup without suppressing strict warnings.

The successful workspace receipt exposed a separate reporting defect. JUnit measured 423.370
seconds of test execution, while the executor measured 856,663 ms including compilation. The
unchanged cadence budget is 840 seconds, so the receipt exceeds it by 16,663 ms. The executor's
2,400-second deadline is only a hard stop: finishing before that deadline does not satisfy the
stricter routine-CI budget. The older aggregator checked outcome and identity but missed this
cadence condition, allowing a successful yet over-budget command to look green.

A new regression first demonstrated that a one-millisecond overrun was silently accepted. The
aggregator now rejects it, while accepting the exact boundary and the preceding millisecond. The
ordinary 0.74 lane also invokes the existing receipt-aware fast-suite checker explicitly; applying
it to the downloaded original receipt rejects that receipt for the recorded overrun. No budget,
noise allowance, timeout, workload or estimator was changed to make the run pass.

Both negative executions remain evidence under their own source identities. The second run's old
28 ship-ready labels are not trusted: they precede the admission ceiling as well as the cadence
repair. Local lint and regression success cannot be combined with that older execution to create
a new green receipt. The next validation must test all repairs on one exact source in ordinary CI.
Until then, cadence compliance remains unproven, the candidate remains unresolved, and expensive
qualification remains disabled. These operational timings describe verification infrastructure,
not HydraCache throughput, native API regression or allocation improvement.

The final local preflight uses one clean source and the pinned 1.94.0 toolchain. Linux passed the
whole ordinary-CI strict lint matrix, including no allocator, system allocator, jemalloc and
mimalloc configurations at the same target scopes. Windows passed strict xtask library/test-target
lint; 109 unit, seven fast-suite, 36 performance-contract and 12 aggregator tests passed. Linux
again passed 207 supervisor tests with one opt-in systemd check ignored. The expected-red canaries
and both rejected ship-admission aliases confirm that cleanup did not open a release boundary.
This closes the reproduced local lint defects, but does not establish the hosted cadence or product
non-regression. The retained preflight explicitly separates local checks from the next hosted run.

### One green source closes tooling defects, not product qualification

The next ordinary execution now supplies the missing exact-source proof. Run
[37538251858](https://github.com/javaquasar/hydracache/actions/runs/37538251858), attempt one,
completed successfully on clean `3ce743508a283a2a0e5612852e9331d9cc65381b`. All thirteen executed
jobs passed: this includes the strict pinned-toolchain lint matrix, public API, MSRV, memory
contracts, migration conformance, documentation and downstream HC/2 admission. The workflow
ran only ordinary checks; costly reference, diagnostic, nightly and soak lanes remained disabled.

The full workspace gate executed 3,765 passing tests and skipped 85 across 477 binaries. The
receipt's monotonic duration was 444,319 ms, including compilation, below the original 840-second
cadence ceiling. JUnit's 229.848 seconds measures the narrower test-execution phase. Both numbers
are operational observations, not API throughput or allocation measurements. The 2,400-second
hard deadline and 1,680-second aggregate budget also remain unchanged.

Why is this useful? The repaired registry separation, ownership review, license metadata, lint,
cadence enforcement and ship ceiling coexist in one successfully checked source. We no longer
need to speculate whether individually passing repairs can run together in ordinary CI. Yet
comparing this duration with the earlier 856,663 ms would still be a misleading speedup claim:
the sources and test sets differ, and cache state and runner characteristics were not controlled
as counterbalanced pairs. One passing run also does not estimate routine-CI variance. The
fast-suite registry still has zero measured baselines; no historical threshold was moved.

Evidence verification did not stop at the green UI. We downloaded the published ZIP, independently
matched its SHA-256 to GitHub's digest, and verified nested JUnit bytes and SHA-256 against the
executor receipt. The original report and both canary outputs retain that same clean source.
The first local report verification rejected missing canonical JUnit and stale local canaries;
that diagnostic was preserved rather than hidden. Only actual verified downloaded files were
then materialized, with previous generated canaries backed up. No receipt or original report
was rewritten to repair identity. Rechecking on the unchanged source passed the cadence checker
and rejected both spellings of `--require-ship`.

The resulting report has 28 `FastGreen` rows and no gated-green or ship-ready rows. Its explicit
reason still says candidate identity and release qualification are incomplete. W11/W12 canaries
remain expected red. Those 28 rows include honest rejected/no-win proposal dispositions: they
are not 28 implemented optimizations. Earlier failed runs stay attached to their original SHAs,
and this successful receipt stays attached to `3ce74350`, not subsequent documentation commits.
The retained record is
`docs/testing/performance/0.74/local-runs/w12-ordinary-ci-exact-source-green-3ce74350.json`.

The distinction determines what can happen next. Candidate composition still has zero accepted
product proposals and C74 remains unresolved. Candidate-specific native/reference measurements,
long-run qualification, packaging, SBOM/advisory/license and supported-target evidence, and the
immutable final archive remain separate obligations. Ordinary green CI removes reproduced tooling
blockers; it does not supply these proofs or authorize expensive workloads. The confirmed outcome
here is more trustworthy verification and admission reporting, not a numerical product gain.

### A closed gate needs a visible reason, and a green tool is not a candidate

The next audit found a narrower W12 defect: release-wide blockers were in JSON, but not in the
human-readable CLI or Markdown report. Row-local failures were printed correctly. This distinction
matters when ordinary checks are green and rows have no local error: the candidate/qualification
boundary still applies to the entire release. The existing command rejected ship requests, so this
was a visibility defect rather than an admission bypass.

We added two regressions before repair. A clean synthetic Git fixture with no failing work-item
rows isolates the report-wide reason as the sole rejection cause. It checks both release aliases
and both normal-generation and ship-request modes, without writing into the real checkout. A
separate Markdown fixture requires every global reason above the row table, retains row escaping,
and compares the generated JSON with the original report value. The legacy no-global-reason layout
is kept byte-identical. These are deliberately synthetic rendering/CLI tests, not green performance
receipts. An initial test compilation diagnostic was also retained: the integration test cannot
access a crate-private constant, and now asserts the external message without exposing a new API.

The implementation only prints existing reasons and adds a Markdown section when they are present.
It changes neither report schema nor admission predicates, row stages, exit codes or numerical
thresholds. The full scoped local set passed 167 tests: 111 unit, 13 aggregator, 36 performance
contract and seven fast-suite tests. All-target checking, strict all-target/all-feature lint and
formatting passed. Clean-source real CLI checks then confirmed the reason in CLI, Markdown and
JSON for all four combinations: normal generation succeeds, ship requests reject, and ship-ready
rows stay zero. Both canaries remain expected red. The source-bound evidence is retained in
`docs/testing/performance/0.74/local-runs/w12-report-reasons-local-2056c151.json`.

The W10 audit puts this useful tooling repair in perspective. All product investigations currently
have negative, no-win or deferred dispositions; accepted proposals remain zero. There is nothing
to compose into C74. The Linux syscall study confirms per-reply write amplification, but does not
supply a different mechanism that resolves the previous shallow-tail and backpressure failures.
A further W3 experiment must begin with a genuinely different design and preregister its semantics
before candidate measurements. Repeating rejected implementations, selecting a documentation-only
SHA as C74 or loosening a guard would not complete the approved plan. Its explicit no-win route is
to publish the measurements and defer the product changes.

Thus two conclusions coexist: ordinary tooling has become more reliable and candid, while a product
candidate and its qualification still do not exist. The earlier hosted success stays attached to
its exact source; it cannot be reused as CI proof for this later fix. No new product or expensive
workload was launched. What remains is a product-direction decision, not an unexplained red tool.

### W3: test the permission to execute, not merely the permission to write

The next investigation asks whether a different adaptive mechanism can resolve the backpressure
failure without penalizing pipeline one. A tempting idea is to check socket readiness or write the
first byte of a response, then execute more buffered commands and coalesce their replies. The
question is semantic before it is numerical: what exactly permits the next mutation?

The canonical generic `AsyncWrite` path executes one command, awaits the complete response write,
awaits flush, and only then dispatches the next buffered command. A SET can already be visible
while its own reply is blocked. That does not permit the second SET to become visible. Here
completion means the writer's completed write and flush, not a TCP acknowledgement, peer receipt
or durability guarantee. Substituting acceptance into a new application buffer would silently
move the existing boundary.

We first passed the seven existing adversarial/adaptive regressions on baseline `8bf4eb26`, then
added two deterministic tests without modifying the server. Both SET requests and QUIT arrive in
one read. A gated writer accepts each possible prefix of the first five-byte `+OK\r\n` response,
from zero bytes through the entire reply, and then returns pending. Explicitly polling the serving
future establishes the blocked state; no sleep or elapsed-time inference establishes correctness.

At all six cuts, exactly one mutation is visible. Another connection reads the first value and a
miss for the second, proving exact state and its own progress while the original writer is blocked.
Releasing the gate permits the second SET and the exact three ordered replies. The complete-reply
case is particularly important: all response bytes can be accepted while flush is still pending,
and the second mutation must still wait. The second test makes that flush fail with BrokenPipe.
The first mutation remains visible, but there is no second SET, second reply or automatic retry.

This eliminates two convenient but invalid permission signals: a writable hint and partial byte
progress. Postponing flush after a full write fails the same contract. Completing write and flush
before continuing is valid, but the previous response write has already happened; collecting
already-executed SET replies before that completion recreates the rejected design. These results
are a narrow feasibility finding for that mechanism family, not a proof that every possible
transport-specific optimization is impossible. The finite test enumeration also covers one SET
reply shape, not every command and transport.

Staging mutations until a shared response write completes is a different architecture, not a
small IO patch. It needs explicit authority, visibility, audit, quota, acknowledgement and native
lock ownership analysis. GET-only specialization likewise cannot assume purity: expiry,
authorization and accounting remain shared obligations. Neither avenue is implemented or admitted
here, and no earlier candidate is reopened. A genuinely different mechanism must establish its
semantic source of savings before a new D2 performance experiment.

The resulting review is frozen in `w3-delivery-frontier-review.toml` with the original deep-pipeline
and pipeline-one thresholds unchanged. A new xtask regression checks both policy equality and
the explicit absence of candidate authorization. Clean source `849fea3a` passed 152 redis-compat
tests with 23 existing opt-in cases ignored and all 37 performance-contract tests. Scoped all-target
checks, strict all-target/all-feature lint and format passed during development; the local contract
passed and W11/W12 canaries remained expected red. Development diagnostics are retained too:
formatting needed repair, and the first contract test exposed a repository-relative design path
that was corrected without changing product behavior or a threshold.

The exact source-bound record is
`docs/testing/performance/0.74/local-runs/w3-delivery-frontier-review-849fea3a.json`.
No new throughput, tail latency, CPU/op, allocation or memory measurements were taken. The RESP
runtime, native/shared hot path and dependencies are unchanged; published 0.73 is untouched. The
earlier hosted green execution remains evidence for its original source, not this review. Accepted
product candidates remain zero, C74 unresolved and admission closed. The useful result is a
stronger semantic screen and a narrower design space, not an invented numerical win.

### W3 staged execution: hidden work still has visible dependencies

The delivery-frontier review left one architectural question: could a connection prepare mutations
privately, coalesce predicted replies, and commit at a compatible boundary? We assessed that idea
locally with explicit authorization to design, not to weaken semantics or implement a product
executor. This is still **not distributed transactions**, nor a store or durability redesign.

An overlay hides the second mutation, but it does not freeze the world. While the first SET reply
is blocked, a direct native caller may invalidate that key, replace its value or consume the
remaining tenant quota. Time may also reach an expiry boundary. A prepared result can therefore
be wrong even if no speculative value was ever visible.

Four new integration guards make those dependencies concrete. Each explicitly polls the canonical
RESP future to every cut of its first five-byte reply, including pending flush after all bytes.
Intervening requests use `dispatch_verified_request` directly; the translator only supplies the
existing binary-safe key identity. Native invalidation makes the queued supported `SET NX PX`
succeed. Native replacement makes the queued GET return the new value. Advancing the injected
clock to exact expiry makes NX succeed and proves that its new TTL begins at execution, not at
earlier preparation. Finally, a native PUT claims the remaining quota while RESP is blocked:
the queued SET must fail with the exact quota error, no third mutation and exactly one rejection
audit. Early reservation would instead reject or delay the legitimate native write.

All 24 controlled trace instances pass. They demonstrate those precise interleavings and native
progress, not a throughput or latency measurement. They also do not exhaust mutable authorization,
event publication, idempotency, cancellation or multi-key atomicity requirements. Development
diagnostics were retained: the first helper mistakenly treated the translated-command enum as a
plan, and an initial bare-NX fixture correctly hit the existing syntax error. Matching the enum
and using the supported NX-plus-TTL shape fixed the fixtures without changing runtime behavior.

A small order model exposes the remaining shared-write problem. For one fixed pre-encoded batch,
the first flush follows the batch's start; the second commit must follow that first flush; but
the second success must already be committed before an arbitrary writer can expose all supplied
bytes. No chronological ordering satisfies all three conditions. Of six enumerated permutations,
three describe a chronological shared write and none retain both guards. The canonical sequence
passes as a positive control because its second response uses a later, separate write. This is
an executable model of explicit assumptions, not a universal transport-impossibility theorem.

The design options now have clearer costs. Commit before shared IO breaks the mutation frontier;
commit after predicted success introduces early acknowledgement and an unretractable failure
window. A store lock or quota reservation across a slow reader moves the cost into native admission
and progress. Apply-and-rollback cannot erase observations or safely restore over a native write.
Preparing privately and revalidating after flush can preserve semantics, but no longer supplies
the proposed fixed-batch syscall reduction. Transport-specific dynamic writers are outside this
model and need their own completion/error/TLS proof and real syscall accounting before a proposal.

We therefore do not implement staged batching for 0.74. Its architecture assessment, explicit
non-admission and unchanged numerical policy live in `w3-staged-execution-review.md` and `.toml`.
Clean source `4caa3b69` passed 157 redis-compat tests, with 23 existing opt-in cases ignored, and
90 targeted xtask tests. Scoped all-target check, strict all-target/all-feature lint and format
passed; the local contract passed and both admission canaries stayed expected red. The record is
`docs/testing/performance/0.74/local-runs/w3-staged-execution-review-4caa3b69.json`.

No runtime, native/embedded path, dependency, threshold, qualification input or frozen 0.73 artifact
changed. No new allocation or product performance numbers exist, and no historic CI receipt was
rebound. W3 remains negative, accepted product proposals stay zero, C74 remains unresolved and
admission closed. The assessment narrowed a plausible architecture into concrete counterexamples;
it did not turn permission to investigate into permission to change the contract.

## A measured large-response owner without response batching

The rejected batching designs do not imply that every output-path optimization
is impossible. They rule out those designs under the existing execution frontier.
A distinct question remains: can we avoid allocating a fresh encoding buffer
while still executing, writing and flushing each command in exactly the same
serial order? That question changes allocation ownership, not acknowledgement or
mutation timing. It does not justify a consuming reducer, native protocol change
or cross-connection buffer pool at the same time.

We first measured the owner rather than implementing all those ideas together.
The prior small-payload W4 assessment remains negative; large GET replies require
their own evidence. The public encoder currently builds a fresh `BytesMut` and
transfers its ownership into a `Vec`. That transfer is already not another payload
copy. A reusable scratch could avoid repeated fresh output allocation, but it
would still copy bytes into the encoding buffer. Claiming both savings before
measuring either would conflate two different costs.

The standalone profiler therefore gained two matched controls. One runs the
canonical server command; the other runs the same command and its existing RESP2
encoder. Each has its own identically initialized store, preload, warmup,
namespace, key, payload and request-ID sequence. GET validates the exact returned
payload; SET validates the exact success. Encoded frames are checked against a
prebuilt header and existing payload slices. We also reconcile dispatch counts,
mutation counts, checksums, final value and cardinality outside the windows.

Validation itself needs attribution discipline. An initial zero-allocation test
ran beside other tests using the process-wide allocator and charged 75 unrelated
bytes. We retained that failure and moved the validator assertion into a filtered
child process. The budget remained zero; changing it would have hidden a harness
problem. Six profiler tests now cover malformed results, large payloads, binary
keys, exact state reconciliation and allocation-free validation.

Before execution we committed a five-cell contract: GET/SET at 256 B and 4 KiB,
plus GET at 1 MiB, seed 740074, three fresh-process repetitions in rotating order.
Small and medium cells use 10,000 operations per window; the large cell uses 500.
All fifteen processes ran from clean source `cc0b2fbf` with the same release
binary. We kept every receipt, with no retry or selected-best sample. Workload,
request, key and payload hashes, as well as allocation totals, match in every
repetition of each cell.

The results identify a large-response owner. GET encoding adds 4,105 B/op at
4 KiB, about 31.39% of the measured canonical execution-plus-encode path. At
1 MiB it adds 1,048,588.168 B/op, about 33.325% of that path. GET at 256 B adds
264 B/op, about 16.97%. SET adds only eight bytes at either tested size. Thus a
large GET has an allocation owner worth investigating; SET's small success frame
does not justify rewriting shared storage or moving cost into native requests.

One detail remains deliberately visible: at 1 MiB the canonical subtraction
exceeds isolated encoder allocation by 84 bytes over each 500-operation window.
All three repetitions contain the same residual. We record it as unassigned
control overhead rather than asserting that every byte of the subtraction
belongs to the encoder. The evidence is
`docs/testing/performance/0.74/local-runs/w9b-serial-encoder-owner-cc0b2fbf.json`;
its referenced raw receipts preserve the timing fields and original output hashes.
A regression checks all registered cells and repetitions, identities, hashes,
checksums, operation counts and deterministic gross allocation totals.

The denominator is crucial. These controls include command construction,
dispatch, store access, reduction and encoding; they exclude decoding, socket IO
and connection scheduling. The percentages are not complete RESP allocation
fractions and certainly not measured throughput improvements. Counting-allocator
timings and coarse Windows CPU samples cannot establish a product CPU claim.
Three baseline repetitions identify an owner; no optimized candidate, p99,
retained-memory improvement or syscall reduction has been measured.

The next isolated D2 design is serial scratch reuse only. It must preserve
complete write and flush before executing the next buffered command, keep the
canonical pipeline-one path, and release scratch before waiting for more input.
It must not increase idle or input/output retention bounds. Partial writes,
disconnects, slow readers, large/mixed replies and RESP3 need explicit tests, as
do the existing native interleavings, expiry and quota guards. A semantically
admitted candidate then needs at least five counterbalanced independent D3 pairs,
the unchanged 20% affected end-to-end allocation floor, and separate native,
ClientSurfaceState and embedded non-regression measurements.

No product runtime, qualification identity, threshold or frozen 0.73 artifact
changed in this attribution step. No rented-host or expensive workload ran.
Accepted proposals remain zero and C74 unresolved. The justified conclusion is
not “scratch reuse made the release faster.” It is narrower and useful: large
GET encoding is now a repeatably measured owner, and a single bounded hypothesis
can be tested without retrying the rejected batching architecture.

## Implementing one bounded hypothesis is not accepting a release win

That D1 owner now has a local D2 implementation. The preregistered guard/policy
commit is `4c5b16d7`; the isolated implementation is `b8cc7c6c`. It is compiled
only by the default-off `experimental-resp-serial-scratch-074` feature, not
silently installed as a production default. No store, reducer, native path,
public codec, input compaction or acknowledgement policy was optimized alongside it.

The mechanism is narrower than a general output cache. A private buffer belongs
to one received-read iteration. Only actual successful GET bulk replies between
4 KiB and 1 MiB qualify, and only at the same consecutive payload size. The first
GET uses the original writer. The second allocates scratch; the third and later
reuse that capacity. We intentionally give up the first two allocation savings
to avoid duplicate decoding, lookahead or changed pipeline-one output ownership.
Each response still finishes its own write and flush before the next command.

Size/shape changes release the buffer. AUTH, HELLO, subscriptions, mutations and
QUIT release it before execution; malformed input releases it before its error
reply. The read-iteration owner drops before another read or event/idle wait,
and drops with the connection future on cancellation/error/close. There is no
global pool or principal-shared buffer. With the pinned encoder, capacity stays
within 1,048,588 bytes; repeated binary response tests confirm the same address
and exact output for RESP2 and RESP3, including overwriting prior contents.

The transport tests must exercise reuse, not just the new branch's first
allocation. They therefore block the third large GET at partial-write and
pending-flush boundaries. Native PUT continues during that pause, but the queued
SET does not execute. Disconnect, failed flush and cancellation leave that SET
absent. Exact injected expiry while GET 3 is blocked makes GET 4 a miss rather
than a stale reused reply. Another native PUT occupies remaining quota during
the pause; the later SET receives the exact error and one rejection audit.
One-byte-read controls never activate scratch and provide an actual canonical
writer comparison for AUTH, subscription, malformed input and QUIT transitions.

The same care applies to the oracle. Existing RESP3 HELLO encoding uses a hash
map with unspecified entry wire order. An initial byte-order assertion failed
on the baseline; the corrected test decodes that map and compares its contents,
while GET/miss/QUIT bytes remain exact. We did not stabilize HELLO encoding as an
extra product hypothesis. A separate fixture correction made the gate distinguish
the first completed flush from the later blocked flush. Both diagnostics remain
in the record rather than being counted as candidate failures or hidden passes.

Clean source passed 166 default and 171 feature-on RESP tests, with 23 existing
opt-in cases ignored in each build, plus 18 feature-on server lifecycle tests and
94 targeted xtask tests. Scoped check, strict lint, format, contract/governance
and expected-red canaries pass. Windows once refused to relink an executable
held by the concurrent canary; after its normal completion the checks ran
sequentially on the same source. This was a validation conflict, not a restarted
performance sample. The retained evidence is
`docs/testing/performance/0.74/local-runs/w9b-serial-scratch-semantic-b8cc7c6c.json`.

The main unresolved tradeoff is memory. A saved scratch buffer remains alive
during the next canonical request and response reduction. That can lower gross
allocation while increasing instantaneous live memory. Bounded capacity and
Rust-owner drop are not proof that RSS, allocator retention or peak memory got
better. Likewise, a stable pointer is not a measured allocation percentage;
payload copies and per-command writes/flushes are still present.

D3 must now preregister exact source/feature/binary identities, matched feature-off
and feature-on runs, independent AA controls, at least five counterbalanced
pairs, the unchanged 20% affected end-to-end allocation floor, separate native
surfaces and peak/idle/RSS checks. Ordinary feature-off CI does not certify the
feature-only helper; its explicit command needs enrollment and evidence before
promotion. No numerical candidate comparison, rented-host workload or qualification
has run in this step. Accepted proposals remain zero and C74 unresolved. We have
implemented a testable hypothesis, not published a product improvement.

## Fewer allocated bytes can still mean a larger live peak

The subsequent W9b D3a screen turned that memory warning into a measured rejection.
Instrumentation and policy were committed at `3171d02a` before any candidate
sample. Two release binaries from the same clean source differed only by the
serial-scratch feature. Seven fixed cells each ran five independent A/A and five
counterbalanced A/B pairs: 140 fresh-process attempts, no failures or retries.
The exact corpus/oracle was prepared outside the measured window; the complete
canonical decoder, dispatch, reduction, encoder, scripted writer/flush and close
were inside. This avoids crediting the candidate against temporary allocations
made merely to construct the benchmark's expected answers.

For 4 KiB GET at pipeline 50, gross bytes/op fell from 15,176.952 to 11,236.152
(25.97%). For 1 MiB GET at pipeline 10, they fell from 3,147,349.520 to
2,308,479.120 (26.65%). Both cleared the unchanged 20% allocation floor.
But the peak of outstanding requested layouts above the starting live owners
rose from 25,134 to 29,239 bytes in the first cell (16.33%), and from 2,114,091
to 3,162,679 bytes in the second (49.60%). All five paired values reproduced
exactly, as did the A/A ratios of 1.00. The five unaffected cells stayed at 1.00.

The incremental peak is exactly one encoded frame: 4,105 or 1,048,588 bytes.
Keeping that frame buffer alive across the next canonical execution saves a later
allocation but overlaps the next reduction's transient owners. The buffer still
drops before the next read, and next-read/post-close owner increments match the
baseline. This is not a leak. It is a live-memory tradeoff that the preregistered
zero-increase peak guard explicitly disallowed.

The conclusion is bounded to a requested-layout profiling allocator and
immediately-ready scripted RESP2 plaintext IO. It is not a socket/RSS, CPU,
latency or goodput claim. A degenerate interval on deterministic repeated counts
does not establish certainty on a production scheduler or physical memory.
RSS endpoints and lifetime peaks remain supplemental, not retention proof.
Native, real-transport, security/concurrency, unprofiled timing, idle/allocator
and hosted-feature-CI qualification were not run after this early rejection;
unchanged native source is not a substitute for a numerical native control.

The candidate is rejected and selectively removed, preserving the full negative
record in `docs/testing/performance/0.74/local-runs/w9b-d3a-3171d02a/` and the
expanded explanation in `w9b-serial-scratch-d3-result.md`. No threshold was relaxed,
no outlying pair removed, and no reducer optimization added to rescue it. D1 was
right about the allocation owner; D2 was right about serial semantics; D3 was
right to refuse this particular ownership lifetime. Those statements can all
be true without an accepted product optimization. C74 remains unresolved.

## Measure the duplicate value, not just the encoded frame

Removing the scratch candidate did not remove the opportunity to investigate
another owner. The next question was narrower: how much of a large GET is spent
copying the client response into the Redis response, before encoding even starts?
The canonical private executor collects client responses and calls a public
borrowed reducer. That reducer clones `Option<Vec<u8>>` for GET. Its original
response stays alive while the new bulk value is constructed. This is a different
hypothesis from retaining encoded output between commands.

We preregistered a new D1 screen at `2562f6f7`, without changing product code.
Six fixed cells covered empty/64-byte/4-KiB/1-MiB hits, a miss and a SET control.
Three rotating repeats produced 18 fresh-process attempts, all successful with
no retries. The request plan, binary key and seed-740074 expected payload were
prepared outside the windows. Public APIs measured dispatch-only, borrowed
reducer-only, and dispatch-and-reduce on separately initialized client surfaces.
Exact byte comparisons and distinct nonempty pointers identify payload duplication;
dispatch/mutation counts, final value and cardinality reconcile independently.

The repeated result is exact: reduction allocates one payload per nonempty GET,
with one successful allocation call. That is 64, 4,096 or 1,048,576 additional
requested bytes/op. Empty hits, misses and SET reduction allocate zero. Combined
minus dispatch gross totals equal the isolated reducer totals in every sample,
with no unassigned allocation residual. For 4 KiB, dispatch and combined gross
are 4,432 and 8,528 bytes/op; for 1 MiB, 1,048,912 and 2,097,488. These fixed-plan
totals exclude decoding, translating, server request IDs and encoding; they are
not interchangeable with previous end-to-end RESP measurements.

The lifetime observation is equally important. For 4 KiB GET, the response-live
checkpoint is 4,198 bytes above the window start; after reduction it is 8,294.
For 1 MiB it is 1,048,678 then 2,097,254. The extra checkpoint owner is exactly
one payload. But whole-window peaks increase from 4,432 to 8,294 and from
1,048,912 to 2,097,254, not by a full payload: earlier dispatch metadata contributes
234 temporary bytes that are gone at the response checkpoint. For 64-byte values
the dispatch peak dominates both windows. Adding stage maxima would misdescribe
the real overlap, even though the extra allocation is perfectly repeatable.

All windows end at their starting requested-live owner counts. This establishes
temporary-owner release in this tool, not physical RSS return or an idle-retention
claim. The tool uses an instrumented System allocator, one synchronous client,
a deterministic injected clock and a prebuilt plan. It does not establish CPU/op,
goodput, p99, production expiration behavior or native numerical nonregression.
The native client's own value materialization still exists; no store or protocol
representation was changed to make this response owner disappear.

The useful next hypothesis is therefore *ownership transfer*, not buffer reuse:
consume a successful GET response in the private executor while preserving the
public borrowed reducer and every fallback. It needs a separate D2 contract,
semantic/property/transport tests and D3 baseline/candidate pairs before any
product claim. We have not implemented or accepted it in this D1 step. Scratch
remains rejected, earlier terminal decisions remain intact and C74 stays unresolved.
The full reproducible record is
`docs/testing/performance/0.74/local-runs/response-owner-2562f6f7/`, with the scope
and next boundary in `response-reduction-attribution-result.md`.

## Transfer the response owner without moving the native boundary

The subsequent D2 step turns that attribution into one isolated experiment,
not a release claim. Before implementation we committed policy at `bf42f776`
and canonical integration fixtures. The private executor already owns its vector
of client responses, whereas the public reducer accepts a borrowed slice. We
can consume the former without changing the latter. At `2379698d`, an empty,
default-off `experimental-resp-get-owner-074` feature selects a private reducer.
Its only admitted shape is one initial GET, no followup, one successful nonempty
value response, with response Vec capacity exactly equal to length. Everything
else delegates to the existing borrowed reducer, preserving error details and
all other command semantics.

The capacity condition matters. A borrowed clone creates a length-sized owner;
blindly transferring a larger-capacity Vec might keep excess capacity alive
during encoding. We tightened admission at `d07c35f0`, before product mutation,
and tested spare-capacity fallback directly. This is a narrower eligibility
rule, not a relaxed memory threshold. After admission `std::mem::take` moves the
existing Vec into `RespValue`; the envelope keeps an empty, allocation-free Vec
and drops normally. There is no cross-command reuse, connection scratch, pool,
borrow across await, encoder change or deferred write/flush. Native value
materialization and its stable public protocol owner remain unchanged.

The tests prove mechanism and semantic equivalence separately. Pointer tests
show that the private result owns the original admitted payload while the public
borrowed oracle has its own clone. Independently executed plans compare binary,
empty, missing and large responses, exact RESP2/RESP3 bytes, expiry and retained
values after replacement/removal, and multi-key ordering/counts. Seven private
tests include all error codes and malformed plan/result shapes; a 128-case
seed-740074 property corpus exercises both ownership branches. An initial
property expectation incorrectly assumed generated Vecs had no spare capacity.
It failed on a legitimate fallback. We corrected the test oracle to reflect
the already sealed admission rule, not the implementation or seed, and retained
that diagnostic instead of hiding it behind a green final count.

Local default and feature-on suites each passed 178 RESP tests, including the
19 transport-adversarial cases; 23 existing opt-in cases stayed ignored. Both
server builds passed 18 Redis lifecycle cases. Check/strict lint passed for the
affected packages and direct dependent. Feature-on ordinary CI is enrolled at
`b3f9d2c4`, but enrollment is not a hosted execution receipt. The exact runtime
overlay guard reconstructs the old canonical fingerprint; source isolation is
not a measured native nonregression claim.

The expected opportunity is removing the attributed reducer copy for admitted
GETs. Its end-to-end magnitude and lifetime effect are still unmeasured. D3
needs a separately sealed finite A/A and counterbalanced A/B allocation/live
screen, including every failed attempt, before any sample. The 20% large-GET
allocation floor, zero peak increase, owner-release checks and four independent
native surfaces remain mandatory. Timing, sockets/TLS, concurrent clients and
RSS/idle/refill retention cannot be inferred from pointer tests or a profiling
allocator. Unlike rejected encoded scratch, this proposal does not intentionally
retain one frame during the next command; whether that yields an admissible
product improvement is for measurements to decide. No D3 gain is claimed yet,
no earlier rejection is reopened, and C74 remains unresolved.

## A removed payload allocation is not yet a throughput result

D3a subsequently sealed the contract and new instrumentation at `213e9e0a`, before
any numerical sample. Unlike the old scratch experiment, the wrapper forwards
only the private GET ownership feature. Release off/on binaries share the exact
clean source; build/source, compiler, lock, contract, binary and workload hashes
are checked. Nine fixed cells run five independently started A/A pairs followed
by five alternating A/B pairs each. Empty hits and misses join SET as genuine
fallback controls. Every expected byte is checked during scripted writes; final
values/cardinality and exact dispatch/mutation/error counts reconcile outside
counting. All 180 attempts succeeded, without invalidation, retry or pair selection.

The complete connection-path window now includes decode, translation, dispatch,
reduction, encoding, write/flush and close. GET 4 KiB/p50 falls from 15,243.952 to
11,147.952 gross requested bytes/op: **26.8697% less**. GET 1 MiB/p10 falls from
3,147,376.520 to 2,098,800.520: **33.3159% less**. Each operation removes exactly
one successful allocation and one payload-sized requested layout, as predicted
by the independent D1 owner attribution. Five A/B pairs repeat each count exactly;
A/A gross/peak ratios are all 1.00. Their preregistered log-scale t(4) intervals
collapse for these deterministic counts, not for production scheduling uncertainty.

The unchanged 20% primary floor passes. Smaller/fragmented controls also do not
regress: 4 KiB/p1 saves 30.2110%, 256 B/p50 6.8763%, mixed 4 KiB/256 B 26.8360%,
and one-byte-read 4 KiB/p10 21.5301%. Payload bytes saved are the same where sizes
match; the denominator differs because framing/fragmentation costs remain. SET,
empty hit and miss gross allocation/peak counts are unchanged. These are not
extra implementation hypotheses or a comparison with historical scratch numbers.

The lifetime guard differs materially from the rejected buffer reuse. Window
peak above start is 25,138 -> 24,600 bytes for 4 KiB/p50 and 2,114,095 -> 2,113,563
for 1 MiB/p10: no increase. Every paired next-read and post-close owner delta is
zero; measured close returns to starting requested-live bytes. The encoder still
needs an encoded frame while holding the value, so removing one transient reducer
copy cannot be assumed to remove one whole payload from peak. The tiny peak
differences are measured, but not independently attributed to specific metadata.
We do not turn owner release into an OS page-return claim.

The result is useful and bounded: this hypothesis clears the early allocation
and requested-live screen; the scratch hypothesis did not. The full 362-file
packet is retained byte-for-byte at `local-runs/get-owner-d3a-213e9e0a/`, with
raw hashes and exact offline replay. There is no reason to rescue scratch or
alter its negative decision. There is also no reason yet to default-enable the
new feature: scripted RESP2 plaintext IO and a profiling System allocator say
nothing sufficient about native capacity, socket goodput, CPU/op, scheduled p99,
TLS/concurrent clients or timed RSS/allocator idle-retention/refill.

Those require a separately sealed phase B with all four independent native
surfaces, unprofiled timing, real transport/security/concurrency and memory cohorts.
Hosted feature-on semantics still need their own current-source execution receipt;
an earlier green ancestor or CI enrollment is not one. No phase B or costly
qualification ran in this step. The feature stays off, accepted proposals remain
zero and C74 unresolved. The detailed scope and all nine cells are recorded in
`docs/testing/performance/0.74/get-response-owner-d3-result.md`.

## Turning instrumentation off is not the same as unprofiled timing

The next phase starts with a methodological audit, not a throughput claim. The
existing native and RESP profilers collect useful attribution, but both depend
on loadgen's global counting allocator. A product instrumentation switch disables
stage counters; it does not remove allocator callbacks. The RESP TCP profiler
also counts socket writes with a wrapper. Comparing those timings could hide or
distort the very CPU trade-off that allocation reduction needs to pass.

The isolated `get-owner-controls-074` workspace therefore builds four variants
from the same clean source: owner off/on with the platform default allocator for
timing, and owner off/on with tool-only requested-layout counting for allocation.
There is no loadgen dependency or socket counter wrapper in this tool. Timing
receipts report gross allocation as unavailable, not zero. Allocation timing is
diagnostic only. Existing measurement tools and their immutable evidence are not
rewritten to fit a new story; the separate dependency lock is sealed with the new
comparison, never mixed with historical binaries.

A finite local B0 contract precedes data. Ten cells cover embedded encoded GET,
ClientSurfaceState GET/SET and unwrapped RESP2 plaintext loopback TCP GET at one
or eight logical clients. Five A/A and five counterbalanced A/B pairs per lane
give 400 fresh processes. Fixed keys/values/seed/work counts, two runtime workers,
warmup, production clocks and exact byte validation are identical across builds.
READY/GO placement happens before warmup. CPU/goodput includes colocated task/
connection setup and measured validation, not preload or receipt serialization.

The unchanged native guard floors apply independently; lower confidence bounds
are used for goodput, upper bounds for CPU/p99/allocation costs. Every pair stays
in its log-scale interval. Too-short CPU windows, excessive background CPU or
A/A noise invalidate and stop the series without retries, threshold changes or
selecting fast pairs. If local precision proves insufficient, that is an honest
measurement limitation, not permission to pronounce the candidate nonregressing.

The latency label matters just as much as the allocator label. A p1 exchange has
closed-loop operation latency; p50 measures the whole batch. Dividing that batch
time by 50 would not recover tail latency for scheduled operations. Exact
scheduled latency and coordinated-omission analysis remain a separate required
cohort. Likewise, requested-live layouts are not allocator retention or RSS.

At preregistration this instrumentation has no numerical B0 result. HC1/HC2,
matched mTLS/RESP3, concurrency 32/128, misses/errors/slow readers/size transitions,
allocator active/resident/retained and timed idle/refill, plus current-source
feature-on hosted CI remain mandatory. This local step neither accepts the
proposal nor enables it by default; it makes the next comparison capable of
falsifying it without moving the cost into native API or overstating evidence.
The precise contract and scope live in
`docs/testing/performance/0.74/get-response-owner-phase-b0-design.md`.

The first sealed launch at `18ee8cdc` illustrates why those rules matter. Its
500 ms prelaunch CPU sample was 57.8125%, above the preregistered 10% background
ceiling. The runner stopped before spawning the benchmark process. All three
packet files survive, with raw hashes and an independent offline refusal audit;
there are no numerical pairs, no throughput/native guard result and no retry.

This does not falsify ownership transfer. It falsifies the suitability of that
launch interval for the planned timing comparison. The D3a allocation result is
still bounded evidence, not suddenly stronger or weaker CPU evidence. A later
quiet-looking snapshot cannot explain the earlier interval, and we neither stop
unrelated applications nor relax the threshold to create a favorable sample.
Before another numerical cohort, quiet conditions must be explicit and separately
preregistered, retaining the refused series. Scheduled per-operation timing and
independent HC1/HC2 harness work can continue locally without a performance claim.

## Preserve offered work before interpreting latency

The next local implementation addresses that methodological gap without running
a new numerical series. It reuses the existing fixed-rate loadgen calendar and
Target interface, but not the old unbounded executor queue or counting allocator.
Every operation retains its original scheduled timestamp, actual target start
and terminal outcome. A stall therefore appears in scheduled response latency,
even when the service duration of later operations looks unchanged. Service
latency is still useful attribution, but it is a separate histogram, not a
substitute for the time a scheduled user operation waited.

Bounded instrumentation must also tell the whole story about lost work. A full
queue is an explicit admission rejection; an expired queued operation cannot
start with a fresh timeout. Drain cancellation records incomplete work with a
censored lower bound, never a made-up response or a zero-latency sample. Success,
error, execution timeout, queue timeout, rejection and incomplete accounting
conserve all offers. Good fraction uses all offers; goodput uses the elapsed
original-offer-through-drain interval. Response percentiles always accompany loss
counts, because a histogram of completed responses alone cannot describe every
offered operation. Histogram overflow is explicit, and nanosecond units do not
claim nanosecond Windows clock accuracy.

Independent native controls now exercise real production HC1 routes over HTTP
and HC2 listeners over gRPC/mTLS, without a daemon or system-service change.
The fixed seed, binary key/value corpus and digest are identical, but each control
owns a separate store. GET/PUT byte oracles, misses, 1 MiB values, tenant separation,
anonymous HTTP rejection and foreign client-CA rejection check that the fixture
is not gaining apparent efficiency through a semantics or authentication shortcut.
Closing clients, joining the listener and checking HC2 resource accounting are
part of fixture completion, not assumptions about cleanup.

These transports do not have identical security contexts: HTTP header identity
is not gRPC mTLS identity. They must be tested as independent off/on controls,
not pooled into a transport-speed comparison. Client-slot mutexes, task ownership,
timestamp recording and response validation also cost CPU/allocations. Removing
a profiling allocator does not remove observer cost; a future preregistration
must keep those costs identical across variants and state its timing boundary.

Virtual-time fixtures confirm that queue stalls are visible, bounded overload
preserves the schedule, deadlines include queue delay, cancellations release
owners and forged/incomplete records cannot become response evidence. Real
native fixtures confirm transport semantics at one/eight slots. None of this
establishes throughput, CPU/op, native nonregression, allocator retention or RSS
improvement. The earlier B0 refusal and bounded D3a result retain their original
meaning. Scheduled RESP response matching, real 32/128 native concurrency,
matched-security cohorts, retention/refill and current-source hosted execution
still precede any full D3 decision. The implementation and remaining limits are
documented in `get-response-owner-scheduled-controls-design.md`.

## A cancelled RESP waiter is still a wire owner

The next implementation makes scheduled timing concrete for RESP2 GET. One
real TCP connection retains each request's caller sequence and its actual FIFO
wire ordinal, then records a complete-frame observation for every parsed reply.
Pipeline 1/10/50 are outstanding-request ceilings, not batches whose time can be
divided to manufacture operation latency. The original loadgen calendar remains
unchanged; frame-observed latency and byte-validated operation completion are
separate boundaries. Parsing an early reply is useful attribution, not permission
to exclude validation/wakeup cost from the operation's SLO.

Cancellation is the subtle part. RESP has no response request id. If a caller
times out after its request was written, removing its pending slot lets the late
reply satisfy the next caller. The bounded actor therefore retains a FIFO
tombstone and its permit until reply or connection close. A late parsed reply
keeps the driver's timeout/incomplete outcome; it cannot become a good success.
Wire owners are drained before elapsed accounting ends. A drain failure is loud,
not a successful receipt with hidden background work. A fresh run also needs a
fresh control, so old response history cannot be rebased into a new sample.

Deterministic fixtures exercise every byte split of binary, empty, null and
error replies, malformed/oversized lengths, partial writes, a gated slow reader,
disconnects and read progress while a later write is backpressured. Distinct
responses verify FIFO association after cancellation. Real loopback fixtures
check each original offer at pipeline ceilings 1/10/50 and a 1 MiB response.
The large preload exposed a fixture frame-limit mismatch: a 1 MiB SET value plus
its envelope does not fit the default 1 MiB request ceiling. Only the local fixture
ceiling changed; no product setting or performance threshold was relaxed.

Independent native HC1/HC2 GET/PUT fixtures now also cover 32/128 client slots,
with barrier-synchronized invocation, HC2 connection accounting, byte oracles
and resource shutdown. These are semantic and ownership checks, not throughput
measurements. They do not supply native nonregression, secure RESP3, allocator
retention, RSS or hosted CI proof. No new numerical series or retry of B0 was
run, and the private product feature remains default-off.

## More sockets do not create one larger FIFO

The next local extension asks a narrower correctness question before measuring
concurrency: can the observer preserve request ownership when there are many
independent streams? The fixture opens 1/8/32/128 real loopback TCP connections
to one production RESP server and shared fixture store. Each socket keeps its
own bounded parser, pending owners and ordinal. A record's wire identity is now
the pair `(connection_id, wire_ordinal)`, not ordinal alone.

Original sequence modulo connection count fixes routing before any response.
Response arrival cannot choose a faster socket or move work to a different
queue. The validator checks FIFO and timestamp monotonicity on each socket,
not across sockets: ordinal zero may legitimately occur on every connection.
Forged connection ids and changed routes fail. A deterministic cancellation
fixture shows that holding one socket's late reply does not consume another's
permit or assign its response to the abandoned caller. The whole group still
gets only one five-second wire-drain deadline.

SET now complements GET, but deliberately writes the same fixed binary value.
This lets the oracle check exact acknowledgement and final bytes through every
socket without pretending that concurrent conflicting writes have a predefined
total order. The semantic grid spans both operations, all four socket counts
and pipeline ceilings 1/10/50, with a separate 1 MiB SET check. It does not prove
mixed-command ordering, MSET atomicity or a throughput improvement.

No product optimization is introduced here. The useful result is a more precise
observer and ownership oracle for later paired measurements. RESP3/mTLS,
multi-key workloads, native nonregression and retained/RSS idle-refill evidence
still precede a full allocation/performance decision; B0 is not retried and
historical numerical packets are not reinterpreted.

## A multi-key command is not a network pipeline

The next observer extension distinguishes two batching dimensions: outstanding
commands on a socket and key positions inside one command. MGET with 128 keys
still has one scheduled offer, one complete-array response and one latency sample.
Reporting 128 successes or dividing its latency by 128 would silently change the
denominator. The adapter instead records actual response item count separately
and rejects drift between the declared batch and the observed reply.

The array parser borrows bulk/null entries and refuses nested or unsupported
types, excessive item counts, integer overflow and total encoded-byte overflow.
The old per-connection buffer ceiling is not multiplied by keys. Null and empty
values remain distinct; positional binary/missing replies and every byte split
are checked. Cancellation retains the entire unfinished array's FIFO owner.
All this observer work still costs CPU and allocations, so functional success
does not itself establish a cheaper product hot path.

The production batch limit is 128, although the plan also names 256-key cells.
Rather than raise a default to make that cell green, the local fixture sends an
oversized MSET and verifies that neither existing nor new keys changed. Other
TCP checks establish last-write-wins for duplicate MSET keys, duplicate-counting
EXISTS versus deduplicated live DEL, and whole-pair visibility during concurrent
MSET/MGET. Scheduled DEL only uses an absent key; it is not evidence for live
removal throughput or a mixed write/delete workload.

The outcome is an observer suitable for further semantic work, not a batch
optimization or numerical claim. The 48 small scheduled cells and bounded
concurrent pair test do not replace expiration/quota/event/fault coverage,
matched native batch measurements, RESP3/mTLS or retained-memory qualification.
Neither historical rejection nor the requirement for repeated paired evidence
is weakened by adding more locally passing tests.

## Native batch is not automatically an atomic MSET control

The next prerequisite was to observe native batches independently, with the same
binary dataset, bounded batch sizes and one original offer per invocation. The
adapter now uses direct ClientSurfaceState, production HC1 HTTP and HC2 gRPC/mTLS
as three distinct controls. Direct dispatch creates no listener or certificate,
but still pays the tool's slot mutex, envelope/key construction and result oracle.
Neither direct dispatch nor a passing functional fixture is a measured native floor.

This work exposed a semantic difference that a pooled throughput chart could hide.
HC1/direct BatchPut validates a surface batch and commits it under the existing
atomic path. HC2's existing batch handler instead invokes single-key operations
in order. Executable counters show one surface dispatch versus one per batch item.
Its default SDK limit also differs: 256 items are allowed there, while the surface
batch limit is 128. Raising one limit or silently treating both as atomic MSET
would make the comparison easier to pass and less meaningful.

The fixtures preserve that distinction. An oversized HC1/direct BatchPut must
leave the original value and all new keys untouched. HC2 accepts a 256-item read
batch but rejects 1025 items locally before dispatch; a mixed PUT/unapplied-CAS
example retains its earlier successful write. Ordered positional results,
duplicates, missing keys, empty/binary values and per-item mutation flags are
validated separately. The scheduled control only records success after its entire
expected result passes; a negative fixture oracle is not goodput.

The outcome is semantic attribution, not a throughput or allocation win: 48 small
scheduled cells and 12 high-concurrency cells establish what each control means.
Future native/RESP comparisons must use equivalent traces and atomicity/security
contexts. A native batch-engine optimization, if justified by profiling, needs its
own proposal and repeated numerical evidence. It cannot be smuggled into the
observer to make RESP gains look like native nonregression. Expiration, quota,
events, faults, secure RESP and retained-memory measurements remain separate work.

## A RESP3 label must follow an observed handshake

Before adding an equally secured native/RESP comparison, the observer needs to
prove which dialect a socket actually speaks. The next local step therefore
adds plaintext RESP3 separately from TLS. Every connection sends HELLO 3 and
validates its metadata before preload and the original-offer clock. Setup does
not inflate goodput; a failed handshake cannot silently fall back to RESP2.

The first real test caught an observer mistake, not a product defect: it assumed
the HELLO map's field order was fixed. The production encoder builds a frame map,
so a legitimate reply can arrive in another order. The corrected oracle checks
exactly seven required unique fields, their types and expected values, while
accepting field permutations. Its 4 KiB buffer, shallow empty-modules shape,
64-byte version bound and timeout prevent a metadata parser from becoming an
unbounded general Redis client. The version string is diagnostic metadata; it
does not attest a binary hash, TLS principal or release artifact.

RESP3 also changes the null frame: `_\r\n` replaces RESP2's `$-1\r\n`.
The connection-local parser refuses the other dialect's null. Empty values,
missing values and binary bytes remain different positional MGET results; an
unfinished array remains one FIFO owner after caller cancellation. A separate
all-miss GET cell keeps misses visible without changing the hit-only workload.
Another real TCP fixture sends HELLO3/miss/HELLO2/miss in one write and checks
that each transition governs the next reply, including bytes already pipelined.

The result is a stronger measurement prerequisite, not a RESP3 speedup. Fifty-six
small scheduled cells, large-value fixtures and parser boundaries establish
dialect/ownership semantics. They do not replace TLS/authentication parity,
subscriptions or full RESP3 conformance, expiration/quota/fault coverage,
native nonregression or retained-memory measurements. No product optimization
is accepted merely because the observer now understands more correct replies.

Clean-source `fc07bf382e6f0c481f381c81b4d1e87108f34db4` passed 57 observer tests
in each default/get-owner variant and 108 focused xtask tests, alongside scoped
formatting, strict lint and local documentation/contract checks. The retained
`docs/testing/performance/0.74/local-runs/get-owner-resp3-checks-fc07bf38.json`
summary includes the initial failed order assumption, bounded virtual-time
handshake timeout and exact source/blob references. Those counts establish
verification coverage, not a throughput or allocation improvement.

## Encryption does not make two security contexts equivalent

The attempt to add a matched secure RESP control stopped at a capability audit,
not at a slow benchmark. The production Redis acceptor constructs server-auth
TLS with `with_no_client_auth()` and server certificate/key material. HC/2's
factory also consumes a client trust root and requires a client certificate.
A rediss client without a client certificate can execute AUTH/cache commands in
the existing production lifecycle fixture. That is the documented TLS plus AUTH
behavior; it is not an mTLS acceptance receipt or a newly discovered product bug.

The distinction changes what a negative test actually proves. A rediss
wrong-server-CA failure proves that the client refused the server. It does not
prove that the server rejected a foreign client certificate. Plaintext rejection,
wrong-password rejection, NOAUTH before dispatch, HELLO AUTH and connection-local
authentication are separate established controls. None can be renamed into
required peer-certificate authentication because their handshake costs and
authorization boundaries differ.

RESP AUTH installs the listener's configured identity; its username does not
select another tenant. Certificate trust is also not a substitute for an explicit
application authorization rule. A tool-only TLS wrapper would test that wrapper,
not silently upgrade the production acceptor. Source-aware contract guards now
retain the capability gap and refuse to waive the equally secured comparison.
Native self-baselines remain valid under their own configuration, while a
cross-security RESP/HC2 row remains absent.

Required RESP mTLS would therefore be a separate opt-in product security proposal:
approve trust roots, certificate requirements, identity/tenant binding, connection
and handshake bounds, compatibility and rollback first; then prove missing and
invalid certificates and denied principals fail before dispatch. This is not a
throughput optimization and cannot be smuggled into observer tooling. The audit
leaves the release's numerical floors, product defaults and qualification identity
unchanged. TLS/AUTH-only controls and the remaining local semantic/memory work
can proceed independently, but they do not close the missing mTLS proof.

The clean-source audit at `b747502d4b63c4e0cdf1d1ee3857f439367f3545` retained
12 selected production TLS/AUTH/RESP fixture passes, one HC/2 security/tenant test
in each default/get-owner variant and 109 focused xtask test passes, with scoped
lint/documentation checks. Exact product and guard blobs and the unmet policy
decision are recorded in
`docs/testing/performance/0.74/local-runs/get-owner-security-audit-b747502d.json`.
No secure RESP adapter or numerical series was implemented by this audit.

## Adding the missing security boundary without claiming a speedup

The user subsequently approved a separate production security extension. We
preregistered the policy before implementation: an explicit inbound client-CA
option, required certificates, preserved AUTH/listener tenant binding and
bounded handshake/connection ownership. This is not a performance hypothesis;
its purpose is to make a future equally secured comparison possible. The earlier
audit is still correct for its pinned source and remains an immutable receipt.

The daemon now has an opt-in `redis_api.mtls_client_ca_path` path using the
standard mandatory client-certificate verifier. With no option, the existing
server-auth TLS/plaintext paths stay unchanged. The new mode refuses dormant or
authless configuration and invalid CA material at startup; it never inherits
the global CA implicitly. Trusting a client certificate grants only transport
access. Redis AUTH is still required before data commands, and neither the
certificate subject nor the AUTH username chooses another tenant.

We measured no throughput or allocation series in this step. Instead, real
loopback TLS fixtures use the production factory and accept loop. They exercise
RESP2/3 HELLO AUTH, binary values and misses; missing, foreign, expired, future
and wrong-EKU client certificates; wrong server CA/hostname; denied credentials
with zero dispatch; and two listener tenants over a shared backend. TLS 1.3
client-side handshake completion is not treated as server acceptance: invalid
clients must produce no successful RESP reply and no backend dispatch.

Resource ownership is part of that security contract. The CA bundle is bounded
to 256 KiB and 16 certificates, the handshake to five seconds and the owned task
group to 128 connections. A 129th connection closes before dispatch; a released
slot can be reused. Virtual-time deadline tests and real-socket shutdown tests
verify cleanup even for incomplete handshakes and authenticated fragmented RESP
frames. Forced shutdown can yield TCP reset or TLS truncation, not a fabricated
successful response. These are new-mode resource bounds, not adjusted performance
thresholds or a change to legacy socket scheduling.

The failures sharpened the tests. The initial PKI fixture omitted a global CA
path required by the existing configuration contract; we fixed the fixture
rather than weakening validation. A shutdown oracle initially assumed only TLS
truncation, then admitted the observed TCP reset while retaining the empty-output
and zero-dispatch checks. A 4 KiB binary fixture does not claim a full 1 MiB value
fits the production 1 MiB request-frame budget once framing is included.

What this proves is deliberately narrow: opt-in production mTLS semantics and
bounded local ownership. What remains unproven is equally important: the secure
scheduled observer, matched native/RESP cost, allocation/retention behavior under
that cohort, hosted CI and release qualification. No speedup is inferred, native
floors are not waived, and get-owner is still default-off. Rolling back to an
older binary requires blocking this listener first: an old binary may ignore the
new option and cannot safely enforce its certificate policy.

The clean implementation source `8565dbfd11ae1ce6d8f20146a37058b8bfb77728`
passed 129 scoped server checks, another 11 mTLS checks with get-owner, 57
observer checks per feature variant and 110 focused xtask checks, with strict
lint, formatting and local documentation/contracts. These are semantic and
verification counts, not measured speedup. The
`docs/testing/performance/0.74/local-runs/resp-mtls-product-checks-8565dbfd.json`
summary retains the exact blobs and the failures as well as the passes.

One failure deserves particular care: a pinned observer repeat alongside builds
failed two plaintext preload tests. After builds finished, both feature variants
passed serialized and then normally parallel on unchanged source and deadlines.
That is useful reproduction evidence, not a proven causal explanation of the
first failure. We retain the local host-pressure/preload stability question and
exclude that execution from numerical claims instead of silently calling the
campaign clean. No root/tool lockfile, frozen qualification identity or older
sealed packet was changed to obtain the subsequent passes.

## Secure observer and isolated memory diagnostics

The next local step closes an instrumentation gap rather than accepting another
product optimization. The scheduled RESP2/3 observer now connects through the
production required-client-certificate TLS factory and accept loop. Every socket
authenticates before HELLO/preload, and shutdown joins its owners and checks zero
active production connections. A shared ephemeral PKI and exact binary dataset
can be checked against HC2 using certificate fingerprints. Matching those inputs
does not erase listener-bound RESP authorization, HC2 identity mapping or their
different batch atomicity; cross-surface numeric comparison remains forbidden.

Large payload testing exposed a concrete measurement-tool defect. The TLS writer
could accept plaintext but keep its final record buffered, so a 1 MiB preload
failed. We added an explicit flush state to the FIFO actor, kept reads and
cancellation live while flushing, and retained a response arriving between the
last write and flush completion. A BufWriter over a one-byte duplex transport
now reproduces the requirement deterministically. The same large-payload secure
case subsequently passed with unchanged payload, seed, bounds and deadlines.
The original failure remains evidence; this is not a product batching win or a
passing A/B attempt recovered by retrying.

Memory diagnostics deliberately use a different executable from timing. Each
fresh process owns one public surface, one payload and the fixed 16-key corpus.
It measures preload, 128 GETs, 128 SETs, idle, public DELETE, refill and shutdown.
Preload includes PKI and transport setup; deletion/refill include their read
oracles. System counters report successful Rust allocation calls, gross requested
bytes and outstanding requested layouts, including client/actor/TLS costs.
They cannot be relabeled server-only allocations/op. Process working set is
separate from logical entry/value-byte retention and from allocator retained
memory. Missing allocator active/resident/retained explicitly stays unavailable,
never zero or a passing admission gate.

The finite diagnostic runner requires a clean committed source and exact binary
hash before creating its seal, owns each child process, retains stdout/stderr and
hashes, and stops at the first failure without replacing an old packet. No timing,
CPU or native nonregression claim follows from those allocation epochs. A separate
exact-SHA hosted semantic workflow checks the feature-off/on observer and strict
lint; it does not launch numerical diagnostics or qualification. Full D3 and W10
still require independent native guards and allocator-retention proof before C74
can be composed or frozen.

The committed diagnostic source `d2252012` completed all 30 fresh-process
attempts (five independent surfaces, two payload sizes, three repeats) without
retry. All public DELETE oracles found zero logical entries/value bytes; all
refills restored 16 entries. The full packet, exact binary/source seal, min/max
analysis and feature-off/on semantic logs are retained under
`docs/testing/performance/0.74/local-runs/secure-memory-d2252012/`.

For example, the independent RESP3/mTLS 64 KiB cell records 50,199,184 gross
requested bytes and 7,573 allocation calls over its 128 GET workload calls in
each repeat. Those totals include both endpoints and TLS/actor work, not just
the response reducer. Idle working set ranges 21,544,960–21,962,752 bytes;
post-shutdown requested-live remains 69,665 bytes. Runtime/report/global owners
are still in scope, so neither value is a zero-retention assertion. The
direct/256 working-set range is much wider, 8,884,224–24,371,200 bytes, despite
identical requested allocation totals. We retain the variation without inventing
a causal attribution or treating OS RSS as allocator resident memory.

Thus the useful confirmed result is measurement readiness and checked
delete/refill/shutdown ownership, not an allocation improvement: this finite
packet is feature-off only, has no A/B estimate, and cannot establish native
nonregression. Allocator active/resident/retained remains explicitly unavailable.
The next full-D3 prerequisites are a separate embedded scheduled control,
unprofiled timing executable and a supported retention lane, with unchanged
admission floors and an entirely new sealed finite contract. W10 remains closed.

Hosted semantic run `37697414775` attempt 1 subsequently passed on the exact
`d22520123880a8a67c816c83a7f76448cbf50af9` source: 68 serialized checks in
each feature variant on Ubuntu 24.04 with Rust 1.94.0, plus formatting, check and
strict lint. Raw logs and identity hashes are retained, not just a green status.
This closes the checked observer's hosted semantic gap, not release CI, native
performance admission, allocator-retention proof or C74 qualification.

The next instrumentation source, `d84a1541`, adds a genuinely separate embedded
control: public HydraCache encoded GET/PUT rather than another ClientSurfaceState
call. Its 16-key binary corpus digest is shared with the native fixtures; only
the fixture's string-key representation uses an injective hex mapping. Fixed
logical client slots cover 1/8/32/128, with separate synchronized concurrency,
1 MiB values, value/miss drift and public delete/refill/flush oracles. It opens
no listener, refuses unsupported batches and does not claim batch-wide atomicity.

The exact clean source passed 73 serialized observer checks per feature variant,
five of them specific to embedded behavior, plus strict lint and 62 performance
contract checks. An automatic guard now verifies all 30 earlier diagnostic
stdout hashes and their unchanged unsupported-retention/admission flags. The
embedded cache's approximate entry count remains a semantic sanity check, never
a heap-retention metric. This closes the missing callable boundary for a later
native guard; it does not itself measure that guard or accept get-owner.
Hosted semantic run `37698947738` attempt 1 also passed all 73 checks per variant
on the exact `d84a1541400fce9e40904c68f9824a91c717c0cc` source; its identity,
raw logs and hashes are retained separately from the older 68-check receipt.

A read-only local capability check found Ubuntu under WSL2 and the reviewed
Rust 1.94.0 toolchain plus C/make/CMake tools. The login default is a different
nightly, so future builds must select the reviewed toolchain explicitly. No
allocator-provider proof or Linux numerical campaign was performed, and WSL2
is not an admitted release host. The remaining full-D3 work is an unprofiled,
bounded numerical runner, a complete preregistered A/A-A/B cohort and a supported
allocator-retention lane. W10 still has zero accepted candidates to compose.

The next local instrumentation step introduces `timing-controls-074`, a separate
unprofiled executable for eight independent scalar GET/PUT boundaries. It has no
counting allocator and refuses allocation-diagnostic builds. A fully specified,
strict JSON input binds the corpus, original offer calendar, concurrency/queue,
deadlines, histogram/SLO bounds and warmup; the executable checks its raw binary
hash before creating any fixture and reports both compiled lock hashes. Its
source SHA remains a coordinator-supplied identity, not an independently verified
Git seal. This distinction prevents a binary-shaped receipt from pretending to
prove clean source or reviewed placement/toolchain.

Warmup uses the same operation and fixed bytes outside the measured calendar.
RESP warmup reserves separate setup IDs, so measured sequence zero is still the
first original offer. Whole-process CPU covers the colocated driver/client/server,
byte oracles, wire/task drain and observation projection; it is not server CPU.
GetProcessTimes and Linux process CPU clocks report their units without claiming
that nanosecond representation implies nanosecond accuracy. Driver goodput and
the slightly wider CPU/projection interval remain separately named.

The useful implemented guard is honesty about short measurements: below one
second of CPU or measurement wall, a CPU ratio is explicitly unusable, including
zero CPU from a coarse clock. Errors, undrained tasks and histogram overflow also
prevent usability. These floors were preregistered before implementation; they
are not lowered to make the 10k semantic driver pass. Successful instrumentation
or even CPU usability is still not an A/A noise pass, A/B benefit or admission.

Semantic tests exercise every boundary's GET/PUT, exact corpus and original
samples, warmup-ID separation, binary/config drift and owner shutdown. Deleted
pre-warmup data and an extra leaked owner are negative fixtures, not successful
cleanup claims. During development a synthetic histogram fixture used an invalid
one-unit maximum and failed; the fixture was corrected and the new input boundary
now rejects that shape before setup. Strict lint also required boxing the report
enum. Neither repair changes a product path or historical benchmark threshold.

The numerical campaign remains unexecuted. Two limitations are explicit: 10k
offers may not accumulate enough native CPU at the unchanged quality floor, and
fresh secure processes still generate different private PKI. A separately
preregistered bounded/streaming driver and fixed private certificate inputs are
needed where those limits prevent a valid cohort. Complete miss/error/slow-reader,
size-transition, independent native and supported allocator-retention proofs also
remain open. No speedup or new accepted candidate is inferred, old D3a/B0 evidence
is not retried, and W10/C74 admission stays closed.

Exact clean source `9e79b012a6046db9946da1bb972750d83b966d5b` passed 88
serialized observer checks per feature variant locally, plus two truly unprofiled
CLI checks per variant, 63 contract checks and 13 release-evidence checks. Hosted
semantic run `37742392078` attempt 1 also passed on that exact source under Linux
with Rust 1.94.0, including the separate unprofiled executable entry points. All
local/hosted logs and their byte hashes are retained in the
`local-runs/timing-instrumentation-9e79b012/` packet. Ship admission still fails
explicitly because C74 and release qualification are incomplete. These are
instrumentation proofs, not a new numerical series or performance claim.
The later receipt guard verifies all 23 retained file hashes/sizes and real test
counts; the local contract suite now has 64 checks. It does not relabel the
earlier source-bound receipts as a passing gate for a newer candidate.

## An idle rented host is useful, but is not a measurement gate

On 2026-10-08 the next decision was whether to use the already rented Linux
host while local instrumentation was being completed. The approved first step
was deliberately bounded: read-only inventory and preparation of a short pilot,
not a performance campaign. The source-bound inventory is retained at
`docs/testing/performance/0.74/local-runs/rental-preflight-62114be0/`; the exact
collector commit is `62114be0f5da3218706e30d7424acfb5d0579d07`.

The practical opportunity was confirmed: eight online CPUs, isolated CPUs 1–4,
performance governors, SMT off, about 61 GiB available RAM and Rust 1.94.0 for
the existing build account. Both protected lifecycle snapshots found no active
campaign marker or fixture context. Only the stable supervisor matched the
cache/build/runner process inventory; the runner was inactive. Installed binary
hashes matched the existing provisioning receipt. No service was stopped,
restarted, reprovisioned or updated, and no product workload was launched.

This checks availability **at the observed instants**, not host reservation,
IRQ isolation, future process placement or a finite A/A noise pass. The fixed
two-second CPU sample cannot demonstrate repeatable latency or CPU precision.
Similarly, glibc and an empty dynamic allocator-library inventory do not supply
allocator active/resident/retained telemetry. Hardware/stack profiling access
also remains unresolved with `perf_event_paranoid=4`; changing that policy was
not part of this audit. The System-retention prerequisite remains red rather
than borrowing jemalloc's differently scoped counters or calling RSS retention.

The prepared first pilot asks only whether the existing bounded observer can
accumulate the already frozen one-second CPU and measurement-wall minima.
It has four baseline-only fresh-process cells: embedded, direct client surface,
RESP2 and RESP3 GET, with identical 16-key/4-KiB corpus, seed, 10k original offers
at 5k/s, eight slots and 64 excluded warmup calls. The strict configs are checked
against the real Rust input and corpus code without constructing a fixture.
One invalid observation stops the probe and is retained; a too-short native
sample would motivate a separately preregistered larger bounded driver, not
padding, changed floors, an automatic retry or a product regression verdict.

Execution is still closed until its Linux source/binary/lock/features,
coordinator tree deadlines, host reservation and placement/noise contract are
sealed and the pilot itself is authorized. Secure A/A-A/B additionally needs
fixed private PKI across fresh processes. The later complete five-pair cohort
and supported System-retention lane remain separate work. This is a useful
way to exploit the host without mistaking sunk rental cost for evidence or
reopening the already closed allocator-replacement/purge experiments.

### A quiet host is not an exclusive host

The next preparation step found a control-plane hazard before running a
benchmark. The proposed external host-lock reservation was not compatible with
the installed supervisor: its maintenance first acquires that same lock,
propagates Busy as an error, and the service restarts on failure. This chain
was checked in repository source, including the last observed installed commit;
we did not induce a failure on the host. An absent campaign marker and idle CPU
therefore do not justify holding the lock. A private lock or forged marker would
not provide coordinated ownership either.

We implemented a deliberately closed preparation tool instead. It checks the
complete four-cell input identity, corpus and typed workload hashes; a separate
metadata-only path inspects exact source/tree, cleanliness, lockfiles, declared
features, target, toolchain, ELF and raw binary/build-log hashes. A fixed build
command is printed as data, not run. Eleven offline tests passed on Windows and
eleven on Linux. Their positive binary fixture is fake: a hash-valid inspection
does not prove compilation provenance, and no real Linux release artifact was
built or inspected in this step.

The limits matter as much as the checks. Killing a timed-out metadata command's
leader does not prove cleanup of an arbitrary workload tree after controller
loss. The workload coordinator still needs owned cgroups, hard deadlines and
sealed partial failures. Safe reservation needs an explicitly reviewed
supervisor-owned diagnostic lifecycle, or a separately approved maintenance
procedure; neither is silently implemented by a benchmark script.

This step produced a confirmed preparation result and a source-based safety
finding, **not a speedup**. It prevented infrastructure interference from being
mistaken for a product regression. No pilot, A/A-A/B cohort, retention claim or
qualification was admitted. The exact packet and remaining decision are in
[the coordinator design](../testing/performance/0.74/rental-pilot-coordinator-design.md).

### Reserve through the supervisor, not around it

After explicit approval for local control-plane development, we implemented the
first diagnostic lease/coordinator model. Campaign and diagnostic admissions
share the existing lock only during short transactions; a typed durable
diagnostic marker owns the longer reservation. The campaign maintenance paths
recognize that marker instead of encountering a lock held throughout a workload.
Corruption, ambiguous markers and lifecycle fixtures refuse admission. This
does not make an external flock safe, nor does it change the installed host.

The fixed four-cell sequence persists its start intent before calling a backend.
Recovery after an interrupted start never issues a second spawn. Logical boot
and monotonic time bind controller loss, cell and total deadlines. Invalid or
oversized receipts stop the sequence; a changed cgroup inode, remaining child
or failed cleanup keeps the host reserved. A terminal state receipt must become
durable before reservation release. An exited leader alone is never cleanup proof.

On the clean implementation commit, Windows passed 18 diagnostic and five
existing host-lock tests; Linux passed 20 diagnostic, six host-lock and 19 server
tests. Linux also checks fixed unit-property construction, and a real temporary
supervisor socket fixture exercises all three maintenance paths with no campaign
backend calls. The diagnostic execution/cleanup backend remains a mock: these
are state-machine and integration results, not live cgroup or performance proof.

This distinction prevents a new kind of overclaim. A property list containing
control-group kill and runtime limits is not a deployed, fault-rehearsed
coordinator. Authentication/replay fencing, live identity verification, bounded
DBus calls, raw spool/receipt sealing and controller/supervisor-loss fixtures
still need implementation and verification. No workload was started, no speedup
was measured and no qualification was enabled. See
[the exact-source evidence and remaining boundary](../testing/performance/0.74/diagnostic-lease-local-design.md).

### A retried heartbeat is not a new lease

The next local hypothesis concerned control-plane correctness, not cache speed:
could a lost reply cause a controller to repeat a heartbeat or destructive stop,
extend a lease, or act on a state revision that had already changed? We added a
separate diagnostic request family and tested it without launching a product
process. Ed25519 covers the entire strict request, including nonce, revision,
boot/binary/build-receipt identity, fixed source/preset and repository/run/attempt/
actor. Campaign signatures cannot authorize this new family; arbitrary paths,
commands, durations and workload overrides are not fields of the protocol.

Authentication alone did not answer the replay question. A bounded durable
journal records intent under the same host fence that validates and mutates the
lease state. Completed requests return the original receipt byte-equivalent at
the typed/canonical response boundary, without another heartbeat or stop.
Changed request-id bodies and reused nonces are refused, as are controller
takeover and stale revisions. A crash between mutation and response publication
is explicitly uncertain: neither replay nor a fresh request executes until a
future reconciliation procedure proves the outcome. The journal is not evicted
to make a test pass; exhaustion fails closed. Its digest detects corruption, not
tampering by the trusted journal owner, and responses are unsigned local receipts,
not remote attestations or fresh status observations on replay.

The tests confirmed those local invariants. Eight concurrent requests sharing a
revision produced one mutation. Cached heartbeat replay left the controller time
unchanged. Cancel replay after terminal release did not stop the tree again.
Injected loss during stop retained the intent and reservation; a separate fixture
modeled the heartbeat/response crash gap. Wrong signatures, future/expired
authorization, unknown fields, corrupt or linked journals and capacity exhaustion
were rejected. Real temporary Linux SOCK_SEQPACKET fixtures checked kernel peer
acceptance and denial. All execution/cleanup observations still came from the
mock backend, so these tests do not prove real recursive cgroup cleanup.

Clean source `403696a7` passed 42 Windows and 70 local Linux checks, plus 68
performance-contract and 13 release-evidence checks. Focused check/clippy,
formatting and documentation gates passed; ship admission remained expected-red.
The first development lint caught three redundant borrows after extracting
fenced methods; fixing them did not change any measurement floor or workload.
The [exact packet and remaining boundaries](../testing/performance/0.74/diagnostic-ipc-local-design.md)
retain the source, raw capture hashes, limitations and negative preparation notes.

This step justified a replay-safe local control boundary, **not a performance
improvement**. No allocations or throughput were measured, no live unit or rented
host was touched, and no production route was enabled. The next useful work is
independent build/config trust, bounded raw receipt handling, the live owned-tree
backend and autonomous watchdog/loss fixtures. Those prerequisites keep eventual
performance attribution separate from an unsafe or ambiguous execution attempt;
they do not replace matched A/A-A/B, native floors or supported retention proof.

### A valid hash is not a trusted build

The next hypothesis was again about attribution integrity, not a faster cache:
could a diagnostic controller accept the right source label but the wrong binary,
profile, allocator or config? The previous Python inspection intentionally proved
only metadata/hash consistency. We left its sealed results unchanged and added
a separate local verifier. This avoids retroactively treating an old unsigned
inspection seal as proof that a particular compiler produced particular bytes.

A strict schema-1 build statement now signs source commit/tree, clean-before/
after assertions, Rust/Cargo target/profile, empty baseline features, System/no
counting allocator, exact command and binary/log/lock/four P0 config hashes and
sizes. Its Ed25519 domain is distinct from controller and provisioning requests;
the builder key, builder identity and repository policy must come from outside
the receipt. Both raw receipt and binary digests must match the diagnostic lease.
Changing a signed field fails verification, while re-signing a disallowed source,
feature, command or config does not bypass the fixed-input policy. The Cargo JSON
projection also requires one matching unprofiled release executable and a final
successful build record; missing/duplicate completion, errors and trailing junk
are rejected. Opaque Cargo fields remain covered by the raw-log digest.

The file boundary needed separate tests. A pathname can change between checking
metadata and opening bytes. Linux therefore walks directories without following
links and opens the exact nine-file bundle relative to a retained directory FD.
It refuses writable/special modes, wrong owner, hardlinks, symlinks, special files,
missing/empty/oversized files and extra entries. Binary hashing streams 64-KiB
chunks and compares metadata before/after reading. The returned snapshot owns
the file descriptors; revalidation compares both named inode identity and pinned
bytes. Temporary fixtures confirmed replacement of a file or entire bundle,
in-place mutation and mode/entry drift are refused. A FIFO fixture did not hang;
independent parallel inspections retained separate descriptors.

Clean implementation `70ca60eb` passed nine new portable tests on Windows and
15 on local Linux, together with 42/70 existing integration checks respectively.
The root passed 69 contract and 13 release-evidence tests. Focused check/clippy,
formatting, doc/local-contract/governance/link/book checks passed; ship admission
stayed expected-red. Development failures are retained: missing-module E0432,
ambiguous fixture map type E0282 and two root test failures caused by an unknown
TOML registration table. They were fixed before sealing, without changing a
workload, floor or prior packet. See [exact evidence and limitations](../testing/performance/0.74/diagnostic-artifacts-local-design.md).

The justified result is a tested local attestation/content verifier. It is **not
compilation or performance proof**: positive fixtures deliberately use a fake,
non-runnable ELF, invented Cargo log and test key. A real trusted build procedure
and environment still need review. Read-only descriptor verification also does
not close a later systemd pathname-exec race; immutable installation/start must
be coordinated under the host fence. No real builder key, production route or
host install was enrolled, and no throughput/allocation workload ran. Raw timing
receipt sealing and live owned-tree/watchdog loss fixtures remain next. Keeping
these boundaries explicit prevents preparation correctness from being mistaken
for an accepted allocation hypothesis, native nonregression or full-D3 result.

### An exit-zero report is not a verified receipt

The next hypothesis concerned measurement integrity, not a faster hot path:
could we reconstruct a diagnostic result from its original samples instead of
trusting a convenient success flag or summary? We preregistered a byte-only
verifier before touching the implementation. Its inputs are an externally
checked build attestation, the exact fixed P0 cell, a terminal summary and the
original stdout/stderr. It does not start a workload, inspect the rented host
or change the observer binary.

The envelope's source SHA is coordinator provenance, not independent Git proof.
Likewise, a caller's assertion that a cgroup is empty is not live recursive-tree
evidence. Keeping these statements separate matters: a perfectly consistent
JSON document can still describe a process that never ran. All positive fixtures
in this slice deliberately invent the reports, CPU values and terminal claims.
They test a verifier; they cannot supply throughput or allocation results.

For successful content we require the original four configurations, 10,000
offers and their 200,000-ns calendar. We reconstruct every outcome count, start/
terminal latency boundary, good-success denominator, goodput and good fraction.
We reproduce the pinned three-significant-digit HDR projection rather than
mistaking a sorted raw quantile for the histogram's upper-equivalent bucket.
Overflow counts are independently derived. CPU provider/scope, per-offer and
per-success division, quality reasons and the unchanged one-second floors are
reconciled separately. Scheduler high-water is only range-checked: final samples
cannot independently reconstruct every queue event.

RESP needs another layer. Every successful offer must have a unique matching
wire record, correct deterministic connection, increasing connection-local wire
ordinal, consistent write/response timestamps, the expected GET frame and a
verified byte oracle. HELLO and topology must match RESP2 or RESP3. A cancelled
waiter or invented response cannot become success through aggregate counts.
Failed reports remain retained failures; their partial observations are not
promoted into validated numerical results.

We also separated outcomes that a single green/red bit would conflate. Content
can be valid yet CPU-unusable because it misses the frozen quality floor. The
right response is to record that result, not add offers, pad CPU or retry. A
process failure, report failure, malformed/inconsistent output and unproven
terminal tree are different reasons. Neither an exit-zero report nor a valid
CPU projection authorizes a performance claim.

Original bytes are capped at 8 MiB per stream. A canonical packet binds their
hashes/sizes, build identity, cell, terminal summary, recomputed decision and
closed admission flags. Offline replay recreates the packet from independent
inputs; changing a decision, stderr, cgroup identity or even harmless leading
whitespace in stdout breaks the original packet binding. Duplicate JSON keys,
missing nullable fields, unknown fields and truncated output are refused without
repair. Stderr is opaque retained data, not assumed empty. An oversized stream
cannot be represented as a complete packet; bounded prefix plus overflow evidence
belongs to the next spool implementation.

The local checks exposed preparation mistakes too: a missing `CellIntent`
serializer, a filtered command that ran zero integration tests, and a negative
test incorrectly demanding rejection when CPU wall time increased by one ns.
We corrected the latter to an impossible wall time shorter than observation;
the verifier and quality floors were not loosened. These are retained development
observations, not product attempts or negative performance measurements.

Clean implementation `ceaa8b07` passed 62 Windows and 96 local Linux checks,
83 focused root contract/evidence checks and 23 governance checks, with scoped
formatting, all-target check, strict clippy and documentation gates. The
[captured evidence](../testing/performance/0.74/local-runs/diagnostic-receipts-ceaa8b07/manifest.json)
binds the exact source/tree and three raw test/check logs. Ship admission remains
expected-red. What justified itself is sample-level, byte-bound reconciliation;
what is still unproved is real measurement provenance and durable retention.

Next come pinned spool descriptors, crash-safe no-overwrite publication and
overflow-prefix receipts, followed by live owned-tree/watchdog loss fixtures.
Only later, separately authorized real builds and diagnostic runs can investigate
CPU feasibility, allocation hypotheses and native nonregression. A canonical
in-memory packet is not fsync, filesystem immutability, process cleanup or a
confirmed optimization. No server, product workload or qualification ran here.

## Retaining a receipt without silently replacing its history

The next local W11 slice tested a different hypothesis: a byte-correct receipt
is useful only if publication cannot silently replace an earlier attempt or
repair an interrupted one into an apparent success. This is evidence tooling,
not a performance optimization. We preregistered the filesystem rules and six
fault boundaries before implementation; the observer, P0 inputs, lockfiles,
qualification manifest and earlier packets stayed frozen.

The Linux fixture reader opens absolute paths component by component without
following links. Exactly two private, single-link regular files are allowed.
It retains directory/file descriptors and compares identity, ownership, mode,
size, timestamps and prefix bytes again before publication. A same-length edit
or replaced pathname is not accepted merely because the new file looks plausible.
The fixture owner remains trusted; these checks are not production ancestry or
proof that all real process writers have stopped.

Each stream is read only up to 8 MiB. Overflow retains the original stat-observed
size and a digest of the bounded prefix, never a claimed digest of the unseen
tail. It has no complete inner receipt. Empty stderr, opaque binary stderr,
malformed stdout and failed process claims remain distinguishable and retained,
not retried or converted into valid timing by aggregate counters.

Publication owns one deterministic pending directory per lease and surface.
Files are created exclusively, written, made readonly and synced. The staging
directory is synced before Linux no-replace rename; its parent is synced after.
There is no overwriting-rename fallback. Independent verification checks the
exact three files against an externally retained manifest digest and recreates
the manifest's decision. Even a matching digest does not authorize forged
admission flags, schema or observed-size fields.

We injected deterministic returns after pending creation, each stream, manifest,
readonly staging and rename. Before rename, staging remains and blocks another
publication; it is not automatically deleted or promoted. After rename, exact
replay verifies the same bytes and repeats sync barriers without rewriting or
starting work. Competing publishers produce only one generation. Separate
primitive tests verify that exclusive writes and no-replace rename preserve
existing file/link/directory names.

Clean `88b9b835` passed 62 Windows checks, 105 local Linux checks, 84 focused
root contract/evidence and 23 governance tests. Windows ran zero spool filesystem
tests; Linux exercised real temporary-file syscalls. Formatting, all-target check,
strict clippy and documentation/local-contract gates passed. The
[exact packet](../testing/performance/0.74/local-runs/diagnostic-spool-88b9b835/manifest.json)
retains the three test/check captures, source/tree and hashes. Test-first missing
module errors, formatting and a cfg-target governance refusal are recorded as
preparation observations, not product attempts.

What justified itself is keeping complete reports, bounded overflow and
interrupted publication separate, with no silent overwrite or workload retry.
What these tests do not prove is equally important: logical error returns are
not abrupt process kills or power cuts; successful sync calls are not physical
power-loss validation; a synthetic valid report does not establish real CPU,
latency or allocation provenance. The module has no production caller or host
release authority. Live owned-tree cleanup, independent watchdog, writer
revocation/fencing, uncertain-intent reconciliation and real trusted build/install
coordination remain prerequisites. No host, product workload, allocation
measurement or qualification ran. Ship admission remains expected-red.

## An empty leader list is not an empty owned tree

The next local W11 prerequisite addressed observation rather than termination.
Source audit showed that the existing campaign reader inspects only its root
process list and that its manager may skip process inspection when MainPID is
zero. We did not alter that campaign path or induce a host failure; we built a
separate read-only diagnostic reader with a preregistered contract.

The [Linux cgroup-v2 specification](https://docs.kernel.org/admin-guide/cgroup-v2.html)
defines populated state across a subtree, while a process list may be unordered
and repeat a PID during concurrent movement/reuse. Accordingly, an empty leader
list is not accepted as recursive emptiness. Threaded/invalid groups, duplicate
PIDs and contradictory documents are refused rather than normalized into success.

The reader walks child and grandchild directory descriptors without following
links. It retains original root/node/file identities and documents, inventories
twice and later refuses changed content or replaced names. Budgets cover nodes,
depth, entries, PIDs and document bytes; boundary tests accept 32 nodes/256 PIDs
and reject overflow. The total document budget describes retained inventory,
not total I/O: descriptor revalidation also rereads bounded files. A populated
tree with no visible PIDs remains busy/uncertain.

The kernel entry point is restricted to the diagnostic unit namespace and checks
boot, filesystem type, safe ownership and caller-supplied root device/inode.
That caller is not authenticated here. Temporary fixtures have a separate entry
point and cannot claim kernel origin. Private result construction also prevents
mistaking the read result for the coordinator's authoritative cleanup observation.
No start, stop, signal, cgroup migration, production route or reservation release
was added.

Clean `f555f32a` passed 62 Windows, 117 local Linux, 85 focused root and 23
governance checks, with check/clippy/fmt and documentation gates green. Eight tree
integration and four unit/property checks covered recursive membership, malformed
and conflicting documents, limits, inode/content drift, unsafe filesystem objects,
fixed scope and seeded unordered PID sets (seed 740074). Windows ran zero Linux
tree tests. [Captured evidence](../testing/performance/0.74/local-runs/diagnostic-tree-f555f32a/manifest.json)
binds source/tree and three raw test/check logs; the initial missing-source
test-first failure is retained as a preparation observation.

This justified the recursive observation layer, not live cleanup. Positive tree
documents and PIDs are invented fixtures; no diagnostic kernel cgroup was observed.
A twice-matching empty snapshot is not atomic against a later fork/migration,
original process-generation proof, stopped writers, authenticated unit policy,
exit status or an autonomous deadline. Those need process-generation binding,
bounded manager operations, writer revocation, an independent watchdog and explicit
uncertain-intent reconciliation before backend enrollment. No CPU, allocation,
latency or product improvement was measured, and ship admission remains closed.

## Retain a process reference, not only its PID

The next W11 prerequisite separates a reusable number from an original process.
The existing campaign reader collects several proc documents separately; we did
not replace that path or claim an observed campaign failure. A new diagnostic
reader pins proc descriptors, compares expected boot/PID/start ticks and cgroup,
then retains a pidfd and rechecks the original descriptors before returning.

The [Linux proc documentation](https://docs.kernel.org/filesystems/proc.html)
explains why retained proc descriptors do not redirect to a recycled PID. The
[pidfd API](https://man7.org/linux/man-pages/man2/pidfd_open.2.html) adds a retained
task reference and exit notification. Neither alone authenticates the original
start. We use both with bounded positional reads and zero-timeout polling;
malformed, terminal, changed or uncertain observations fail without PID-only
fallback. Counters and runnable/sleeping state may change, but identity fields
must remain exact. The diagnostic namespace accepts only its fixed root and
bounded descendants, not a prefix-matching foreign unit.

Review found another boundary worth making explicit: pidfd_open and a procfs
mount can interpret numbers in different PID namespaces. We compare the retained
pidfd's fdinfo Pid, as defined by the
[kernel implementation](https://raw.githubusercontent.com/torvalds/linux/v6.18/fs/pidfs.c),
with the pinned proc directory's PID. The bounded readlink of procfs `self` is
used only to locate this reader's own fdinfo. This is a mapping check, not host
namespace authentication or a namespace-creation rehearsal.

Clean `b8c626cf` passed 62 Windows, 130 Linux/WSL, 86 root and 23 governance tests,
with check/clippy/fmt/documentation gates green and ship expected-red.
[Exact evidence](../testing/performance/0.74/local-runs/diagnostic-process-b8c626cf/manifest.json)
retains source/tree and three byte-preserved test/check captures. Thirteen new
Linux tests include seeded parser checks (740074), injected identity/boot/group/
migration/read/poll failures, unsafe document substitution, parallel positional
reads, and real local self/helper pidfds. Closing a non-product cat helper's
stdin ended it; the original handle then refused revalidation and recapture.
Actual PID recycling, migration and reboot were not induced. Windows ran zero
Linux process tests; a strict-clippy preparation failure was fixed and retained
as an observation before the clean repeat.

This justifies the local generation guard, not a live backend. Expected start
identity, executable and systemd policy still require authentication; an exec
can preserve the generation. A matching cgroup path is not a pinned tree inode.
Observations do not persist a terminal failure or survive supervisor restart as
an uninterrupted-lifetime proof: the backend must retain the first rejection.
Bounded manager/tree binding, writer revocation, watchdog and uncertain-ledger
recovery remain. No host service, product workload or performance measurement
ran, and no throughput/allocation win or release admission follows from this step.

## A timeout must bound the operation, not only its caller

The next diagnostic launch prerequisite is control-plane liveness. The existing
campaign dispatcher was left unchanged. Its blocking manager calls cannot gain
an end-to-end deadline merely by setting a method timeout: connection setup and
teardown remain separate operations. Returning from a waiting thread would also
leave the underlying operation alive. This is a design finding, not a measured
campaign failure or a product performance result.

The new diagnostic reader runs inside a fixed helper mode of the same Rust
supervisor binary. The parent sends only a canonical lease/boot/four-surface
scope, clears inherited environment, polls both output pipes under one byte and
time budget, and terminates only its own unreaped helper on failure. It never
executes a supplied command or systemctl. Helper address space, CPU, core dumps
and runtime thread count have separate fixed limits; an unconfirmed helper
blocks another operation in the same client. These are safety ceilings, not
adjusted throughput thresholds or a guarantee against kernel/scheduler stalls.

The [systemd interface](https://raw.githubusercontent.com/systemd/systemd/v257/man/org.freedesktop.systemd1.xml)
and [D-Bus specification](https://dbus.freedesktop.org/doc/dbus-specification.html)
provide the protocol foundation. The implementation uses a fixed root-owned
system-bus endpoint, pins the manager's unique owner with UID 0 / PID 1, checks
each reply's exact sender, and refuses owner or boot drift. Only that owner's
specific NoSuchUnit reply means absent. Two loaded-unit reads must agree;
property caches and foreign cgroup-prefix matches are not accepted.

Local invented properties and non-product child failures test the parsing,
deadline, output and cleanup boundaries. Real system-bus absence checks are a
separate observation: they do not create a diagnostic unit, verify its complete
hardening, authenticate a binary, revoke writers or release a reservation.
Partial helper output remains runtime evidence until the backend durably seals
it. Parent restart also loses the runtime child reference. Controlled start/stop,
live tree/process/artifact binding, first-failure persistence, autonomous
watchdog and uncertain-ledger reconciliation still precede the numerical pilot.
No latency, CPU/op, allocation or throughput improvement follows from this step.

### A green process check was not the end of concurrency analysis

The manager repeat exposed an earlier process-reader weakness: one parallel
revalidation returned Invalid. The positional reader avoided shared seek offsets,
but still assembled a proc document through several syscalls. Source inspection
suggested that a regenerated seq_file document could supply a new tail after an
older prefix. We did not retain the offending kernel document, so that exact
cause remains a hypothesis, not a traced kernel event.

A deterministic test supplied an old complete document followed by a regenerated
tail. It failed before the correction. The reader now performs exactly one read
at offset zero, using 65,537 bytes to detect the unchanged 64-KiB limit, and copies
only the returned prefix. Empty, oversized, Interrupted and malformed results
remain refusals, never partial-document retries or a new PID capture. The strict
parser and original eight-reader, sixteen-iteration test were not relaxed.
This prevents multi-syscall prefix/tail assembly; it is not general atomic-file
or whole-process observation proof. It is control-plane correctness, not a
measured allocation or throughput improvement in either product API.

Another repeat found a fixture ordering race: a test helper could exit before
receiving stdin, making the parent's correct IO refusal precede the exit category
the test intended to check. Fixtures now receive request and EOF first; the
separately adopted retained child has its own mode. Production refusal behavior
is unchanged. The final [byte-preserved packet](../testing/performance/0.74/local-runs/diagnostic-manager-c2fdc3f4/manifest.json)
retains these failures and the final clean-source result: 62 Windows, 154 Linux,
88 root and 23 governance tests, check/lint/fmt/docs green, require-ship red.
Four local systemd absence reads and wrong-boot refusal were observed separately
from synthetic loaded properties. Complete live backend, writer fencing,
watchdog and uncertain-intent recovery are still launch gates, not inferred wins.

### Replace invented build fixtures with actual bytes, without inventing trust

The next step compiled the exact frozen observer `62114be0` for Linux release
with Rust 1.94.0, locked dependencies, no features and no counting allocator.
A dedicated checkout exposed a practical identity risk first: Windows newline
conversion changed lock bytes even though Git reported a clean checkout. That
new, unused checkout was replaced with LF before compiling; existing product and
qualification worktrees were untouched. Frozen locks/configs were checked before
and after. P0 configs are separate hash-pinned inputs, not modifications to the
older frozen observer tree.

Native Cargo JSON stdout was retained separately from compiler stderr. An actual
artifact uncovered a blind spot in the invented build-log fixtures: Cargo used
a valid version-only local package ID, while the verifier accepted only its
name@version spelling. The deterministic grammar test failed before correction.
The parser now accepts those two exact local forms and additionally rejects
foreign source kinds and tool paths; profile, empty features, one artifact and
final-success requirements were not relaxed. See the
[Cargo package-ID specification](https://doc.rust-lang.org/cargo/reference/pkgid-spec.html).

A new bounded, read-only audit inspects the real ELF, native Cargo log, locks and
four P0 configs. Its private result cannot become VerifiedBuild and explicitly
reports false attestation, source/install trust, execution and admission flags.
Windows and Linux inspectors agreed on the actual byte digests. Four invocations
of the compiled observer used --validate only: valid=true, fixture_started=false,
admission_allowed=false. The [sealed build packet](../testing/performance/0.74/local-runs/diagnostic-local-build-62114be0/manifest.json)
retains these outputs, test-first failures and clean-source checks (13 Windows,
19 Linux artifact/reader checks, 89 root and 23 governance tests).

This replaces compilation assumptions with one actual local build and structural
content verification, not independently reproduced or trusted compilation. It
does not produce CPU, allocation, latency, retained-memory or native-regression
evidence. A reviewed trusted builder policy and the safe live backend, original
tree/process binding, writer revocation, watchdog and uncertain-intent recovery
still precede a signed, baseline-only numerical pilot. No default test signature
or unsigned local output may be promoted into that execution authority.

### Build provenance needs its own authority, not just another signature domain

The next question was architectural: reuse the provisioning signer or give the
trusted Linux builder a separate key. The user chose separation. Provisioning
authorizes installation tooling; a builder attests which exact source, compiler,
configuration and artifact bytes produced a binary. Separate message prefixes
prevent accidental protocol reuse, but do not isolate compromise of a shared
private key. Separate keys permit independent revocation and narrower policies.
This benefit depends on a genuinely separate secret boundary, not two files
available to the same arbitrary build script.

We implemented an externally pinned canonical public policy with fixed
repository/builder identity and an independently supplied controller key.
Shared, weak or mismatched keys and altered/noncanonical policies are refused.
A dedicated Linux signing executable checks fixed build observations, ELF/Cargo
content and every lock/config digest before deriving and signing a schema-1
receipt. It does not expose arbitrary statement signing or join the installed
supervisor's execution routes. Private key files require private owned parents,
descriptor-based no-symlink traversal, single links and strict mode; output is
create-new with file/directory synchronization, never replacement.

The manual CI lane separates compilation from signing across hosted runners.
The build coordinator removes credentials and Rust/Cargo/loader overrides from
its subprocess environment, uses a fresh fixed-source checkout, retains native
Cargo output separately from stderr and executes --validate only. The signing
job compiles reviewed tooling before obtaining the separate secret, then checks
this run/attempt's bundle against the external public pin. Human review and the
exact branch restriction are prerequisites, not assumptions inferred from an
environment name. No private builder key belongs on the performance host.

Local synthetic positive/negative tests validate the policy, signing round trip,
content/source drift, private paths and refusal to overwrite. They do not prove
that a trusted production builder ran, establish reproducible compilation, or
measure CPU/allocation/latency. See the [design and retained evidence](../testing/performance/0.74/diagnostic-builder-local-design.md).
The initial environment lookup returned 404; the legacy environment's empty
protection settings were deliberately not copied. After the user explicitly
delegated technical review to the agent, we configured a new environment with
the authenticated javaquasar account as required reviewer, the exact feature
branch as its sole permitted deployment branch, and administrator bypass
disabled. This is delegated account approval, not independent second-human
review. No pending build or signing job has been approved automatically.

The review exposed a specific gap: checking that a reviewer exists does not
prove an administrator cannot bypass that review. The first negative test showed
that the bypass-enabled configuration was accepted before the fix. The coordinator
now requires the
explicit boolean false for can_admins_bypass; missing, null, true and numeric
zero values fail closed. Both preparation jobs use this check. A new Ed25519
seed was generated in transient local memory and sent only to the new GitHub
environment secret through stdin. The controller's private key was neither
read nor reused. Its public key came independently from the installed host
configuration, not an incoming receipt or artifact. The new public policy and
external digest are retained in Git; the private seed is not stored in local
files or on the performance host. Deleting that secret requires a fresh reviewed
rotation, not recovery from this evidence.

The [enrollment packet](../testing/performance/0.74/local-runs/diagnostic-builder-enrollment-20261009/manifest.json)
records protection metadata, public policy validation and local regression
checks. Registration itself does not prove that the stored secret can produce a
valid real build signature: that evidence must come from the protected build/sign
lane and independent verification of its receipt. That lane is still blocked on
workflow registration: GitHub requires a workflow_dispatch file on the default
branch before first manual execution. We have not merged product code or added
automatic triggers to work around it. This closes key-enrollment preparation, not backend,
writer-fencing, watchdog, recovery, installed pilot or performance admission.

The authorized one-file registration became a separate PR based on released
main, not a merge of the 0.74 product branch. Its local and CI attempts exposed
two different barriers. The CI topology contract requires an exact workflow
inventory and correctly rejected the new, undeclared file. Full local verify
then exposed an inherited orchestration defect: its Linux workspace clippy step
passes --all-features, enabling mutually exclusive allocator implementations
together. The product refused that union, including multiple global allocators;
this is not a reason to weaken allocator isolation. The correction belongs in
verification tooling, using separate feature sets and retaining the existing
coverage, not changing product semantics or acceptance thresholds.

The [registration evidence](../testing/performance/0.74/local-runs/diagnostic-builder-registration-5b75c2bc/manifest.json)
retains both original failure logs. Seven coordinator tests passed, but focused
green tests do not make the full gate green. The user subsequently authorized
the CI inventory and verification-tooling correction. Both builder jobs are now
declared manual-protected without new exemptions. Artifact upload and download
names carry source SHA as well as run and attempt, and compiler installation has
its own bounded deadline. The Linux workspace lint now uses default features;
all-feature lint still covers the rest of the workspace, while HydraCache keeps
separate common, System, mimalloc and Linux jemalloc checks. Its compile-time
allocator guards are unchanged.

New regression tests first failed against the old union and missing artifact
SHA, then passed after correction. The registration branch passed 13 verification
orchestration tests, 12 CI-reliability tests, package check and strict lint. The
first CI inventory correction passed topology validation but triggered the
memory baseline canary: the supposedly live inventory was also a digest-pinned
measurement input. Changing its pin would erase the very drift the canary
detected. Instead, the registration branch restores the exact snapshot and
uses a separate live inventory, following the split already made in 0.74. A new
regression checks both the frozen digest and manual-protected builder entry.
Full local verification and refreshed CI subsequently passed. PR 216 merged as
2b3c5794 without admin bypass, and the manual workflow became registered. The
[registration result](../testing/performance/0.74/local-runs/diagnostic-builder-registration-c7cba271/manifest.json)
retains the complete successful log as well as the earlier refusals. This repairs
the path to collecting trustworthy build evidence, not cache throughput. The
separate build/sign review starts on exact tooling 33596da7; installation,
qualification and performance claims remain outside this registration result.

A clean Linux repeat of the 0.74 tooling passed 211 Rust tests and seven Python
tests, with format, package check/strict lint, docs and local contracts green.
Two incomplete full-verification logs are also retained. One candidate was
superseded after the frozen-input canary rejected it. The next debug build grew
its reproducible target cache to about 37 GiB and threatened local disk capacity;
only that cache was discarded before repeating the same tests without debug
symbols or incremental artifacts, with two build jobs as in ordinary CI. Neither
interruption is a cache regression, a performance measurement or a green full
gate. Keeping these classifications distinct prevents an operational preparation
problem from silently changing a baseline, acceptance threshold or release claim.
The compact repeat first hit another environment precondition: an inventory test
calls python, while WSL only provided python3. A dedicated task-tool alias to the
existing Python 3.12.3 allowed the unchanged full gate to pass. The optional console
gate explicitly skipped without Node; successful Rust verification is not browser
evidence. These operational details are retained, not hidden by the green repeat.

### A signed build is still not permission to measure

The [actual protected run](../testing/performance/0.74/local-runs/diagnostic-builder-hosted-37923423889/manifest.json)
subsequently completed on reviewed tooling 33596da7. It compiled the frozen
observer on one hosted runner and signed the checked result on another, with
separate delegated approvals. Before the second approval, review compared the
original ZIP digest, clean source/tree observations, exact compiler/command,
locks, native Cargo JSON, ELF and four fixed configurations. The observer was
called only with --validate: every report said fixture_started=false and
admission_allowed=false. A partial network download was refused and retried
without duplicating the workflow.

After signing, a local read-only verifier checked Ed25519 against the public
policy already retained during enrollment, not a policy chosen by the incoming
bundle. It also checked the actual signed content. Wrong-pin, ELF-mutation and
signature-mutation guards passed, including recomputing the changed receipt's
digest so a signature failure could not be mistaken for a simple hash failure.
A clean-source rebuild repeated the audit and its bounded-file-reader test.

What this establishes is deliberately narrow: a receipt from the separately
enrolled builder attests this new hosted artifact and its checked inputs. It is
not independent-human review, mathematical compilation proof, reproducible-build
evidence or an installation receipt. The older unsigned local binary was not
retroactively signed. No cache workload or qualification ran, so these results
provide no throughput or allocation claim. The next boundary still requires exact
live backend/start binding, immutable installation, writer fencing, watchdog,
durable failure and reconciliation/loss rehearsal before even a baseline pilot.

### SSH access incident before host installation

Preparation also exposed an operational security failure: an SSH command passed
a private-key path as configuration, and the parser printed key material before
connecting. A failed connection does not make a disclosed credential safe. With
explicit approval, a fresh key was created locally and tested without an agent
or fallback identities before the old public key was removed from AX42. New
access and sudo remained usable; the old key was then refused. Existing access
entries and SSH server policy were preserved, and the supervisor was not stopped.
The [sanitized audit](../testing/performance/0.74/local-runs/ssh-access-rotation-20261009/manifest.json)
contains public fingerprints and observed outcomes, not private material.

That result applies only to AX42. The same old key is configured for a previous
rental host whose host identity changed. Connection remains blocked until its
identity is independently verified or the user confirms retirement; overriding
known_hosts would trade a visible blocker for unverified access. Builder and
controller signing keys are separate and unchanged. No observer installation,
cache workload or qualification follows from this credential rotation.

### Binding preparation to policy and the remaining lease

The next backend prerequisite joins the previously separate checks: the
independently pinned builder policy, signed bundle, retained file descriptors,
validated diagnostic state, exact cell intent and fixed unit specification.
The fixed installation reader now requires the checked policy type. The
prepared object retains the original descriptors and refuses any state or file
drift; its first refusal cannot be erased by a later matching observation.
This latch is runtime-only, so it does not replace a durable failure journal or
restart reconciliation. Fixture origin remains visible and cannot certify a
production installation.

The [test-first packet](../testing/performance/0.74/local-runs/diagnostic-start-material-20261009/manifest.json)
also records a concrete lease-boundary defect. In Starting, an old recorded
start time could still grant a 60-second unit runtime after the latest state
observation left only 17 seconds. A deterministic regression failed with
60 instead of 17. Start-only construction now caps both the cell intent and
RuntimeMaxUSec using the latest observation and refuses an exhausted lease.
The running-state model and 60/300-second ceilings are unchanged; the example
uses an invented clock, not a benchmark or an altered measurement duration.

These checks establish preparation consistency, not a live start. Positive
fixtures use fake ELF and test signatures. The backend still needs a fresh
boot/clock check and durable intent under the host fence, authenticated loaded
unit policy and original executable/process/tree identity, writer revocation,
autonomous watchdog and explicit uncertain-request recovery. No performance
claim follows from this safety correction, and no host unit or workload was
started for it.

### Loaded settings and an invocation that cannot be silently replaced

A signed binary and correct unit name still leave two different questions:
what settings did the manager actually load, and is this still the original
execution? A separate bounded read-only helper now authenticates the manager
and compares two uncached observations, including 53 typed settings. The
expected projection comes from the fixed diagnostic unit specification and
validated startable state. It checks resource limits, hardening, environment,
working directory and exact extended command flags as well as argv. Extra
lifecycle commands and environment files refuse rather than become invisible
cost or authority outside the intended observer.

The runtime guard retains the manager owner, boot, object, nonzero InvocationID
and cgroup path. A replacement invocation cannot be adopted merely because it
has the same unit name. Missing or changed settings, absence and observation
failure permanently refuse that guard. This prevents a later successful read
from hiding a failed observation; it is not restart-safe durable reconciliation.
PID, active state and result can evolve within the invocation, so this guard
does not certify original process identity, completion or an empty tree.

The implementation also exposes an important API limit: systemd 255 reports
the append output mode but does not expose its configured filename in the
[execution property table](https://github.com/systemd/systemd/blob/v255/src/core/dbus-execute.c).
Checking the setter's name as if it were a getter would produce either refusal
on every real unit or false confidence from synthetic properties. The projection
therefore verifies the mode and explicitly leaves output-file descriptors,
actual executable and effective process environment for the next binding step.
These are safety checks for trustworthy future measurements, not throughput or
allocation improvements, and they do not open numerical pilot admission.

The [local evidence](../testing/performance/0.74/local-runs/diagnostic-loaded-20261010/manifest.json)
keeps the boundary concrete: 189 Linux and 28 portable Windows checks passed,
as did 115 root contract, evidence and governance tests. Positive loaded-policy
projections are synthetic; four real local system-bus reads confirmed only
absence, and an incorrect boot was refused. These results justify continuing
the process and descriptor binding work, not claiming a successfully launched
diagnostic or a performance gain.

### A stable PID is not a stable executable or output destination

The next safety hypothesis was narrower than a launch: if the original process
generation remains alive, can a read-only observer still detect a different
executable or stdout/stderr target? A pidfd alone cannot answer that question.
An owned helper can exec another program or redirect an FD without changing the
generation checked by the existing process reader. Tests exercise both cases:
generation revalidation succeeds, while the new object guard refuses.

The [object-binding design](../testing/performance/0.74/diagnostic-io-local-design.md)
retains the original proc directory and follows only its fixed executable and
FD 1/2 magic links with O_PATH. It compares the resulting objects with retained
read-only expected descriptors, without reading their content or changing shared
file positions. The executable's metadata must remain unchanged. Output files
must stay distinct, regular, single-linked and mode 0600, with writable append
flags in bounded fdinfo. Their length may grow, but an observed shrink or overflow
of the existing 8-MiB ceiling refuses. Original pidfd, boot, generation and cgroup
checks bracket collection; every failed IO observation latches that runtime guard.

The start-material hook supplies the checked bundle's private executable
descriptor and revalidates exact material/state around binding. Temporary output
descriptors are still explicit caller assertions. The composition positive uses
a cat copy, invented Cargo log and synthetic signature: it tests how these
components join, not the hosted observer's execution, installation or compilation.
Four parallel readers on an appending helper additionally check that growth does
not require shared seek or exact equality of sequential size observations.

One negative preparation result was in the fixture, not the observer: buffered
Rust stdin consumed the byte intended for the helper after exec. A one-byte
unbuffered control read fixed the handshake without changing production checks.
Compiler/lint failures are retained too, including the pinned toolchain's
diagnostic-renderer panic on an ineffective append-open option. Removing the
redundant write flag fixed that warning; no lint allowance or compiler migration
was used to pass it.

The limitation regression matters just as much as the positive: renaming an
output file and replacing its former pathname remains acceptable when the
process still writes to the original single-linked object. This primitive does
not certify filenames. Nor do sequential inode/flag reads prove a single open
file description, unchanged content, uninterrupted executable identity, effective
environment or absence of other writers. Retarget-and-return, truncate-and-regrow
and other changes between observations can remain invisible. Refusal is runtime
state, not restart-safe recovery.

This step removes a specific false-confidence path in future measurement tooling;
it is not an allocation or throughput improvement. Fixed named-output preparation,
authenticated manager/process/tree composition, fenced start/stop, writer
revocation, watchdog and durable uncertain-intent reconciliation remain before
backend enrollment. No service, product workload or qualification was started.

The [retained local packet](../testing/performance/0.74/local-runs/diagnostic-io-20261010/manifest.json)
records the clean 15-test baseline and all preparation negatives. Final checks
passed 200 Linux tests, with one pre-existing ignored; the 10-test IO set also
passed three additional repetitions. Portable Windows checks passed 28 tests
without executing Linux IO readers, and 116 root contract/evidence/governance
tests passed. Formatting, affected package checks/lint and local documentation
and performance contracts were green. The packet keeps working-tree checks
distinct from qualification receipts and preserves the frozen manifest digest.

### An unchanged output inode can still belong to the wrong filename

The preceding regression suggested the next precise hypothesis: can directory
and fixed-name observations reject an output destination swap without forbidding
normal append growth? The answer from local fixtures is yes. The new
[named-output reader](../testing/performance/0.74/diagnostic-named-output-local-design.md)
derives lease, surface and `stdout.json`/`stderr.log` from validated state, opens
each directory component without following symlinks and retains every original
parent/child binding. Files must remain distinct, regular, single-linked and
0600, within the unchanged byte budget. Extra or missing cell entries refuse.

The important comparison uses the same non-product process and original output
object. After moving stdout outside its cell and replacing its fixed filename,
the inode-only reader accepts: its assertion still matches where the process
writes. The composed reader refuses: that object no longer occupies the required
name. Restoring the old name cannot repair a refused capability. Process exit
and checked-material refusal are also propagated rather than hidden by a fresh
reader. This gives a concrete safety result, not a faster product benchmark.

Append requires a different contract from immutable artifact verification.
Freezing file timestamps or exact lengths would reject legitimate output.
Revalidation instead advances length high-water marks and rejects observed
shrink or overflow, without reading stream content or altering shared offsets.
Four independent readers can observe an appending file; unrelated entries in
an ancestor directory do not invalidate its stable identity. Conversely,
directory generation, ownership, permissions and each named child remain bound.
The existing terminal spool verifier keeps its stricter stable-content rules;
it is not repurposed as a live reader.

These observations remain sequential, not an atomic namespace/FD snapshot.
Replacement-and-return between checks and truncate-and-regrow can remain
invisible. Temporary UID/GID assertions do not establish safe production
ancestry, original launch, effective environment or absence of other writers.
The wrapper explicitly requires fixture start material, and production roots
are refused. Fixed-root preparation and manager/process/tree composition must
still join the host fence, stop/revoke writers and durable failure/reconciliation
before a real diagnostic backend is enrolled. No service or numerical workload
was started, and no allocation or throughput improvement is claimed here.

The [local packet](../testing/performance/0.74/local-runs/diagnostic-named-output-20261010/manifest.json)
retains 210 Linux passes, with one pre-existing ignored, three extra repetitions
of all ten new tests, 28 portable Windows passes and 117 root checks. It also
retains the missing-API compile refusal/compiler-renderer panic and the first
static guard's formatting-sensitive failure. Normalizing whitespace fixed that
guard without relaxing any runtime check. Affected check/lint, scoped format,
documentation/local performance contracts, links and book build passed. These
working-tree observations support the narrow fixture safety result, not release
readiness or a numerical performance claim; the qualification digest is unchanged.

### An unchanged invocation can still report a different process

The next safety hypothesis concerns the relationship between three observations.
The manager identifies a unit invocation, procfs identifies a process generation,
and the recursive cgroup reader identifies a stable tree. Independently valid
observations need not describe the same execution. The invocation-only guard
intentionally permits MainPID to change as lifecycle progresses; that flexibility
cannot become permission to substitute a new process for a measurement already
bound to an original generation.

The [original identity join](../testing/performance/0.74/diagnostic-live-identity-local-design.md)
therefore borrows the retained process and tree readers rather than capturing
replacements. It requires the manager's MainPID to equal the original process
PID, keeps the original boot and start ticks, and requires the PID in exactly
the node named by that process's cgroup path. Finding it somewhere in the unit
tree is insufficient: a sibling membership does not establish the same identity.
Frozen nodes and anything other than active/running/success refuse the stable
live observation. These are conservative diagnostic conditions, not a change to
product lifecycle semantics.

The revealing regression holds invocation constant while changing MainPID.
The older invocation-only check accepts, as designed; the composite refuses and
stays refused after the original PID returns. Two manager observations surround
retained process/tree checks, and failures at either position latch the same
guard. Manager deadline failures keep their raw bounded output and helper cleanup
status. A subsequent apparently healthy observation does not erase uncertainty.
No new pidfd, tree inventory or invocation is adopted to make the check green.

The local positive join tests use explicitly invented inputs. An actual owned
cat helper provides a different kind of evidence: its ordinary cgroup and a
temporary document tree cannot be promoted to fixed diagnostic kernel evidence,
and refusal does not kill the helper. Producing a positive live composition would
require the real fixed diagnostic unit and root provenance. That remains a
controlled rehearsal, not something the synthetic matrix establishes.

The retained tree is deliberately stable. A legitimate fork or membership change
can invalidate it, so this guard is not yet a dynamic lifecycle observer. The
checks are sequential and cannot detect every change-and-return between reads.
Systemd's ControlGroup string also does not attest to the caller-supplied root
inode or authenticate original launch. Production output preparation, joining
executable and output evidence, writer revocation, durable first failure,
watchdog and uncertain-start reconciliation remain before backend enrollment.
The result is narrower identity refusal, not an allocation or throughput gain.

The [retained packet](../testing/performance/0.74/local-runs/diagnostic-live-identity-20261010/manifest.json)
contains 220 final Linux passes with one pre-existing ignored, three further
repetitions of the ten new tests, 28 portable Windows passes and 118 root checks.
The seven-test baseline is loaded-manager coverage only: Cargo's filter excluded
the selected tree target. Both complete implementation runs execute its eight
tests. Initial compile/source-guard failures and the earlier alternating order
are preserved; the final order puts both source checks between manager reads.
These working-tree checks and their byte/hash verifier support the scoped safety
result. Positive live composition and numerical performance admission remain
separate, and no frozen qualification input is regenerated.

### Fixed output names also need trusted ancestors

A retained filename is not enough if an untrusted owner can replace its parent
directory. The temporary named-output tests proved replacement refusal, but their
explicit fixture ownership could not establish the production ancestry policy.
The next hypothesis therefore concerns ownership, not speed: require a stronger
fixed-root reader without granting it permission to prepare or launch anything.

The [production output inspector](../testing/performance/0.74/diagnostic-production-output-local-design.md)
derives one diagnostics path from validated lease and cell state. Root owns the
ancestor chain and the private lease directory; the asserted nonroot worker owns
the private cell and two regular single-linked streams.
The lease permits known-path search with mode 0711, but only root can list or
change its entries. Ancestors also require search permission for the worker;
cell and stream privacy remain 0700 and 0600. Unsafe write/special bits,
wrong owners, links, namespace drift, observed shrink and the unchanged 8 MiB
stream ceiling refuse observation. The separate reader exposes revalidation and
refusal status, not file descriptors or conversion into a fixture capability.

All 4096 permission/special-bit combinations are checked for each directory role.
These policy positives are invented metadata, not measurements of a server's
filesystem. The private wrapper test likewise proves propagation of a refused
temporary reader, not production provenance. Existing append, concurrency and
replacement fixtures verify the shared reader behavior. A positive production
observation will still require the real fixed path and independently established
worker account identity.

Review also found a subtle validation gap: reading root metadata once to validate
ownership and again to retain the stamp can retain a different observation. The
implementation now stores exactly the stamp it validated. Later checks compare
that original stamp and parent-child binding; they do not refresh it to recover
from a refusal. The walk remains sequential, so change-and-return between checks
and continuous writer identity remain outside its proof.

This is progress toward trustworthy diagnostic attribution, not an allocation
or throughput improvement. Numeric UID/GID inputs remain caller assertions;
account enrollment, fenced preparation, original-start authentication and joining
executable/output evidence to the live identity guard remain before execution.
The inspector creates no directories, changes no ownership and runs no workload.

The [retained local packet](../testing/performance/0.74/local-runs/diagnostic-production-output-20261010/manifest.json)
contains the initial 150 Linux passes with one pre-existing ignored, three additional
eleven-test repetitions, 28 portable Windows passes and 119 root checks before
the traversal correction.
The initial commands used the default nightly; final controls explicitly use
1.94.0. Their passing behavior tests do not establish a numerical comparison
across toolchains. The absent-API compile refusal and pre-review run are retained
separately, and the frozen qualification digest remains unchanged.

A final compatibility review found that the initially accepted root-owned 0700
lease would block a nonroot worker's WorkingDirectory. [Systemd 255](https://github.com/systemd/systemd/blob/v255/src/core/exec-invoke.c#L4477-L4518)
changes UID before applying that directory. The new test first rejected the old
policy; the correction requires searchable root-owned ancestors and lease 0711.
Only traversal is added, not listing, writes or access to the private cell's
content. This source/policy check still does not constitute a positive unit launch.

The corrected policy passes 151 Linux tests with one pre-existing ignored and
three further twelve-test repetitions. Portable Windows tests remain 28; the
updated root policy guard, affected check/lint and scoped format pass again.
The earlier 119-test root suite stays labelled pre-correction rather than being
reused as full final-source coverage. The useful result is a stricter ownership
observation that also accounts for nonroot pathname traversal; execution and
numerical admission still require their separate gates.

### Independent successful observations still need an execution join

Checking the manager, process, output names and signed executable separately can
leave an important attribution gap: each check may be valid while describing a
different execution. The next hypothesis therefore concerns their binding, not
speed. A single read-only composition must refer to the original process already
held by the manager/process/tree identity guard and the original output/material
objects, without accepting substitute evidence.

The [execution composition](../testing/performance/0.74/diagnostic-live-execution-local-design.md)
borrows the three original guards and obtains the process only from the identity
reader. Its public entry point accepts no second process, arbitrary output FD or
pathname. Full state equality includes revision and clocks, not just lease and
binary hashes. Fixed production origin is required for both named outputs and
signed material before manager observation; a temporary fixture cannot become
production evidence merely by agreeing on those strings.

The existing material hook supplies the retained signed executable, and a private
named-output hook supplies its retained streams. Revalidation checks identity,
outputs and material before original process IO, then checks material, outputs
and identity again. Each identity observation retains the original manager and
process/tree bracket. This establishes a stricter sequential observation, not an
atomic snapshot or uninterrupted execution proof. Per-helper limits remain, but
four manager observations and bundle hashing do not create a whole-operation
deadline or a performance improvement.

The useful regression is refusal that survives destruction of the wrapper. A
first construction or observation failure latches identity, outputs and material,
not only the temporary composite. Subsequent checks do no observations and
restoring a path or the caller's state does not reset those objects. The first
returned error keeps its typed category; manager failures keep bounded output and
helper cleanup status. This runtime latch is not a durable failure record and
does not revoke writers or release a lease.

Private synthetic tests exercise all seven observation positions, stop at the
first error and verify independent guards under concurrency. State tests vary
revision, clocks, cell and identity fields; invented origin flags are used only
to test their private predicates, never for a binding. Actual owned cat fixtures
exercise early origin refusal and the common finish path after original IO
binding. Their invented identity holder and temporary output wrapper remain
negative-only test seams. Neither they nor passing tests authenticate a fixed
production unit, worker account or original launch.

The hypothesis is supported locally at that narrower boundary: retained inputs
can be composed without reopening substitute-process or fixture-promotion APIs,
and refusal remains shared after wrapper drop. Controlled positive production
composition still requires worker enrollment and fenced output preparation.
Original-start authentication, effective environment, writer revocation, durable
refusal, watchdog and uncertain-intent reconciliation remain separate gates.
No allocation, throughput or qualification claim follows from these safety checks.

The [retained execution packet](../testing/performance/0.74/local-runs/diagnostic-live-execution-20261010/manifest.json)
supports implementation `e1dbae9e`: the completed baseline passes 151 Linux tests,
reviewed coverage passes 158 with one pre-existing ignored, and all seven new
tests pass three more repetitions. Windows has 28 portable passes and no Linux
composition execution. The root suite passes 120 checks before packet registration;
affected check/lint/format and documentation controls pass. Every invocation uses
1.94.0. The two missing-API refusals and earlier first green remain separately
labelled. These are working-tree safety observations and retention checks, not
clean-source qualification, positive unit execution or performance measurements.
Ship admission stays closed and the frozen qualification digest is unchanged.

### Output preparation must preserve the host fence

Read-only output inspection cannot safely stand in for preparation. The next
hypothesis concerns cooperative exclusion: compare the exact reserved cell and
prepare its fixed names while retaining the same host lock used by the campaign
and diagnostic coordinators. Production preparation additionally needs enrolled
worker ownership and a root-owned directory capability, so the
[local preparer](../testing/performance/0.74/diagnostic-output-preparation-local-design.md)
deliberately remains a separate temporary fixture API.

The fixture uses the actual non-root effective UID and GID, not numeric caller
assertions or a lookup of `hydracache-perf`. It checks its private 0700 root,
retains the existing no-follow lock object and creates descendants relative to
retained directory descriptors. Exact persisted state, Reserved stage and
asserted boot/monotonic deadlines are checked under the fence. Campaign markers,
both lifecycle contexts and pending diagnostic or IPC documents refuse before
cell preparation. No heartbeat, state revision or frozen workload changes.

Each cell must be new. Its only streams are empty, exclusive-created 0600
`stdout.json` and `stderr.log`; streams and directory entries are synced before
success. Reusable output/lease parents remain private, while existing complete
or partial cells refuse. The API neither removes nor truncates them and returns
only the existing read-only fixture output guard. Failure after cell creation
therefore leaves visible uncertainty rather than silently repeating preparation.

The first broad parallel test run found a Busy refusal during fixture state
setup. A separate deterministic test exposed a concrete lock-lifetime issue:
closing the parent descriptor does not release an advisory lock when a duplicate
of its open file description remains. The test retained a cloned descriptor and
failed before the correction, without forking or starting a workload. The new
fence explicitly unlocks before successful return and propagates that error;
error exits also attempt unlock during Drop. Fixture reservation now uses the
same retained fence rather than relying on a close-only handoff. This explains
the corrected mechanism, not an independently traced cause of the earlier
parallel scheduling event. No automatic retry or relaxed Busy assertion was added.

The [preparation packet](../testing/performance/0.74/local-runs/diagnostic-output-preparation-20261010/manifest.json)
separates the 158-test baseline, ten-test first green, initial parallel failure,
cloned-descriptor red and subsequent correction. Final Linux coverage passes
172 tests with one pre-existing ignored; all fourteen new tests pass three
additional repetitions. Eight concurrent preparers create exactly one cell.
Faults after the cell and either stream preserve partial files and refuse replay;
root, lock, output-parent, lease and cell replacement refuse further writes.
The total-deadline test retains a fresh controller heartbeat so that one timeout
cannot accidentally mask the other. Windows passes 28 portable tests, not Linux
preparation tests; root contract/evidence/governance coverage passes 121 checks.

The useful result is an explicit cooperative fence and exclusive preparation
boundary with retained failure state. It is not production ancestry, worker
account authentication, signed start authorization, power-loss recovery or an
atomic namespace snapshot. The compiler-renderer failure, accidental field
placement error and strict-lint refusal are retained alongside corrected checks.
Production preparation, original-start authentication, watchdog and uncertain
intent reconciliation still precede a controlled positive rehearsal. These tests
do not measure allocations, throughput or qualification performance.

### Kernel credentials are not account enrollment

The next question is narrower than whether a worker may start: does the original
retained process still have the credentials we explicitly expected? An account
name in a unit and UID/GID arguments passed to an output inspector cannot answer
that question. The
[credential observer](../testing/performance/0.74/diagnostic-worker-credentials-local-design.md)
reads `status` relative to the original retained proc directory, without accepting
a second PID or replacing the original document. Original-generation checks
bracket two bounded positional reads; selected fields must match both the policy
and the original projection. The first failure latches refusal, even if a test
restores the substituted descriptor afterward.

The policy checks real, effective, saved and filesystem UID/GID values, exact
supplementary groups and zero inheritable, permitted, effective and ambient
capabilities. It requires NoNewPrivs, but does not confuse that flag with complete
privilege confinement: the [kernel contract](https://docs.kernel.org/userspace-api/no_new_privs.html)
primarily constrains privilege gains through exec. Likewise, the bounding set is
retained and checked for drift, not treated as an active capability set or forced
to zero without a corresponding unit policy. The
[proc reference](https://docs.kernel.org/filesystems/proc.html) distinguishes those
fields. A non-UTF8 process name remains irrelevant to the byte-level credential
projection. Missing, duplicate, malformed, oversized or uncertain inputs refuse.

The [local packet](../testing/performance/0.74/local-runs/diagnostic-worker-credentials-20261010/manifest.json)
retains the 172-pass Linux baseline, initial thirteen-test green and final
188-pass Linux run with one pre-existing ignored. All sixteen new tests pass
three more repetitions. Seed `0x74c2026` drives 256 valid single-field mutations,
so rejection is not explained merely by malformed input. Owned cat helpers
exercise real retained documents, repeated and parallel reads, incorrect
assertions, NoNewPrivs refusal and exit. Private descriptor substitution verifies
that restoration cannot repair a refused guard. Windows passes 28 portable
tests, not these Linux tests; root contracts/evidence/governance pass 122 checks
before packet registration. The invalid target command and diagnostic-renderer
failure remain visible beside the clean pre-implementation API refusal.

The hypothesis is supported for this original-process, point-in-time observation.
It is not evidence of continuous or all-thread credentials, trusted user
namespace enrollment, authenticated account-name mapping or positive production
execution. Fixed `hydracache-perf` enrollment and composition with the existing
execution guard are still needed before signed original-start authorization.
No account, unit, supervisor host, observer or product workload was changed;
these results measure safety behavior, not allocations or throughput.

Before the baseline, six obsolete ignored Cargo caches were cleaned after exact
path, reparse-point and active-compiler checks. Their inventory contained about
12.3 GiB of logical data; C: free space rose from about 1.5 to 13.1 GiB at those
observations. Current caches, raw evidence and all worktrees remained intact,
and the previous packet passed its verifier again. This is operational disk
maintenance, not a performance result; deleted caches are recoverable by rebuild.

### Fixed account lookup needs a bounded helper and a separate trust policy

The next safety hypothesis addressed the other half of worker identity. Kernel
status can confirm numeric credentials of an original process, but it cannot
prove that those numbers belong to the intended `hydracache-perf` account. A
name lookup is therefore needed before enrollment, yet calling NSS directly in
the long-lived controller would put provider latency and allocation on that
controller's path. The
[fixed account observer](../testing/performance/0.74/diagnostic-worker-account-local-design.md)
executes lookup in the existing bounded child envelope instead. No selector,
helper pathname, account creation or worker launch is accepted.

Two complete rounds compare forward and reverse user/group mappings with a
canonical membership list. Each reentrant lookup uses a fixed 16 KiB buffer,
without ERANGE growth or retry; membership capture accepts at most 32 IDs and
rejects duplicates, invalid counts and sentinel values. Only the fixed names,
non-root UID/primary GID and sorted membership IDs leave the helper. A runtime
guard retains the original projection and refuses permanently after drift or
an observation error, preserving the first typed failure and cleanup status.
The existing parent deadline, output budget and pending-child reservation apply
to this new fixed mode.

The distinction between caller and provider memory matters here. A 32-entry
array does not constrain NSS-internal allocation. Child resource limits and the
parent envelope supply the outer limits; the fixed caller buffer is not an
allocation-performance claim. The [GNU libc group API](https://ftp.gnu.org/old-gnu/Manuals/glibc/html_node/Setting-Groups.html)
also includes the primary group in `getgrouplist`, which is not proof of the
kernel supplementary groups configured for a particular process.

The [packet](../testing/performance/0.74/local-runs/diagnostic-worker-account-20261010/manifest.json)
retains the 190-pass baseline, clean missing-API refusal and initial eleven unit
plus three integration passes. Final coverage passes 205 Linux tests with one
pre-existing ignored; all fifteen new tests pass three additional repetitions.
Seed `0x74a2026` changes valid identities rather than relying only on malformed
documents. Review corrected a primary-GID mutation to keep its group list
consistent, then required every changed projection to pass canonical decoding
before testing sticky refusal. The earlier captures remain separate because
their GID cases did not establish that stronger property. The real WSL operator
mode consistently returns a refused observation,
exit 9 with confirmed helper cleanup and empty stdout. Positive name/ID mapping
checks use a private deterministic seam; they are not a real host enrollment.
Windows passes 28 portable checks, with no Linux account tests executed. Root
contracts, evidence and governance pass 123 checks before packet registration.

The bounded lookup and sticky refusal hypothesis is supported within that
scope. Trust still requires a reviewed provider/configuration and namespace
policy, followed by composition with the original process credential guard.
Sequential matching reads neither freeze NSS nor prove all-thread or continuous
credentials. Root-owned production output preparation, authenticated signed
start, watchdog recovery and a controlled positive lifecycle rehearsal remain
before production activation. No product workload ran and no throughput,
CPU/op or allocation improvement is claimed from these safety checks.

Before this baseline, only the 0.74 incremental cache and canary Cargo target
were cleaned. C: free space rose from about 12.6 to 81.6 GiB at those observations;
the current build caches, canary process capture, raw evidence and all worktrees
were retained. These are disk-maintenance observations, not an exact physical
deletion size or a performance result.

## Binding account observations to original kernel credentials

The [asserted binding](../testing/performance/0.74/diagnostic-worker-binding-local-design.md)
closes a consistency gap between a fixed account observation and the numeric
policy already retained by an original-process credential reader. Borrowing
both original guards avoids a second PID capture or replacement policy. UID
and primary GID must match exactly. The union of explicitly asserted
supplementary groups and primary GID must equal original NSS memberships.
This does not infer whether the primary GID appears in the kernel's `Groups`
list: the original credential policy still checks that list exactly.

Mapping mismatch refuses before observation. Successful revalidation follows
account, original credentials, then account again. A failure preserves its typed
first error and permanently refuses both inputs, even when construction fails
or the wrapper is dropped. A later matching observation cannot repair the
original guards. The hypothesis here is that composition can preserve identity
and refusal without converting observed values into authority.

The [local evidence](../testing/performance/0.74/local-runs/diagnostic-worker-binding-20261010/manifest.json)
supports that hypothesis within a narrow test boundary. Against a 205-pass
baseline, final Linux coverage passes 218 tests with one pre-existing ignored;
the thirteen new cases pass three additional repetitions. Seed `0x74b2026`
mutates 256 valid mappings rather than merely malformed documents. Real owned
NoNewPrivs cat helpers exercise original kernel status, exit refusal and
independent concurrent bindings. Their positive account projections are
synthetic, not real NSS enrollment. The public constructor separately covers
mapping mismatch, pre-refusal and pending-helper reservation. Real NSS operator
inspection in WSL still refuses with exit 9 and confirmed cleanup. Windows
passes 28 portable tests, and root contracts/evidence/governance pass 124 checks.

Review replaced a temporary group union with bounded iterator comparison.
That removes an allocation from this comparison's source, but safety tests are
not an allocations/op or CPU/op measurement. Trusted provider/configuration and
namespace policy still precede production use. Sequential checks are not an
atomic or continuous credential proof; root-owned output preparation,
authenticated original start and watchdog recovery remain separate. No product
workload or qualification ran, and no numerical performance gain is claimed.

## Retaining the original process user namespace

Numeric credentials need a defined reader context. The
[user namespace rules](https://linuxman7.org/linux/man-pages/man7/user_namespaces.7.html)
describe how numeric ID mappings depend on the reader namespace. A fixed user
name and matching numbers do not alone establish a trusted host identity.
The [new opt-in guard](../testing/performance/0.74/diagnostic-user-namespace-local-design.md)
retains kernel user namespace objects for the already pinned process and the
current reading thread. It deliberately uses `thread-self`, not the process
leader's namespace. Nsfs and namespace type checks precede device/inode matching;
ordinary namespace-directory lookup remains descriptor-relative and no-follow.

The namespace and original process-generation checks alternate. This catches a
dead original process even though its retained namespace FD can remain valid.
Every observation error or identity drift permanently refuses the guard, and
restoring a substituted descriptor cannot repair it. No PID, namespace ID, path
or FD can be supplied to replace an input; no namespace join or launch operation
is exposed. The hypothesis is a bounded, opt-in consistency check rather than a
conversion of observed namespace equality into production authority.

The [local packet](../testing/performance/0.74/local-runs/diagnostic-user-namespace-20261010/manifest.json)
supports that narrow mechanism. A 218-pass Linux baseline precedes the missing
API check. After correcting a missing trait import, eleven tests pass; review
adds a distinct-kernel-object comparison and the final suite passes 230 with one
pre-existing ignored. All twelve new cases pass three further repetitions.
Real owned cat helpers exercise same-namespace positives, exit refusal, both
namespace descriptors, directory substitution and independent concurrent guards.
Actual mount namespace and regular-file descriptors refuse as user namespaces.
Seed `0x74e2026` mutates 256 synthetic device/inode identities per repetition;
these are not real foreign-user-namespace transitions. Windows passes 28 portable
checks and root contracts/evidence/governance pass 125 tests.

The guard does not certify an initial host namespace, every thread or a trusted
NSS provider. It also cannot retroactively certify the reader namespace used to
open an existing credential document; that binding must precede later
composition. Production output preparation, authenticated original start and
watchdog recovery remain separate. No product workload or numeric allocation,
CPU/op or throughput result follows from these local safety checks.

## Checking namespace context when credentials are opened

Namespace checks performed after a credential document was opened leave an
important gap. Linux [task_state](https://github.com/torvalds/linux/blob/v6.6/fs/proc/array.c)
projects numeric UID, GID and group values through
[seq_user_ns](https://github.com/torvalds/linux/blob/v6.6/include/linux/seq_file.h),
which uses the file's retained credential user namespace. A later namespace
snapshot cannot retroactively establish that opening context.

The [separate opt-in reader](../testing/performance/0.74/diagnostic-namespace-credentials-local-design.md)
therefore pins the original process and current thread's namespace first.
It checks namespace, opens the original status document once, checks namespace
again, validates credentials, and checks namespace once more. Later observations
use namespace, credentials, namespace without reopening status. Both component
guards keep their original generation, descriptor, document and namespace
checks. Any failure stops the sequence and refuses both guards; restoring a
substituted input cannot recover either. No preopened credential reader, new
PID, path, FD or namespace identity can replace these inputs.

The local hypothesis is a safe opening/read sequence rather than a new launch
authority. It preserves the numeric-only credential API and account binder;
no helper or IPC route is added. The bracket does not inspect or attest kernel
file credentials and cannot detect every unobserved transition between reads.
Trusted host/initial namespace and NSS configuration, all-thread policy,
production output preparation, authenticated original start and watchdog
recovery remain prerequisites for production composition. These safety checks
provide no numerical allocation, CPU/op or throughput gain.

The [local packet](../testing/performance/0.74/local-runs/diagnostic-namespace-credentials-20261010/manifest.json)
retains a 230-pass Linux baseline, missing-API refusal, ten initial new passes,
and a 242-pass final suite with one pre-existing ignored. All twelve new cases
pass three further runs. Real owned NNP cat helpers cover original exit, wrong
policy, unhardened credentials, status FD/named-document and namespace FD
substitution, restoration and independent concurrent readers. The refusal of
both guards and preservation of the original status identity support the
bounded mechanism; no production cgroup or foreign namespace transition is
claimed. Seed `0x74f2026` varies 256 modeled first-error/stage cases per run.
Windows passes 28 portable tests and root contracts/evidence/governance pass
126 checks. Raw API captures also retain command-wrapper exit-reporting mistakes;
they are separate from the green implementation tests. Scoped lint/check and
documentation gates pass, while require-ship remains expected red and the
qualification manifest remains unchanged.

## Binding the original account to namespace checked credentials

The next local hypothesis joins the original fixed-account observation to the
reader that checks namespace before opening status and around credential reads.
The [new opt-in binder](../testing/performance/0.74/diagnostic-namespace-worker-binding-local-design.md)
accepts only these two retained guards, not another process, numeric policy,
account projection or descriptor. It reuses the existing UID/primary GID and
group-membership mapping rule and refuses mismatch before any helper observation.
The older numeric-only reader and binder remain unchanged.

Account checks surround the namespace-checked reader. The reader preserves
its original status and namespace/credentials/namespace sequence; no document
is reopened or reinterpreted from a replacement policy. Any error preserves
its first typed details and refuses the account and both privately owned reader
guards. Constructor failure and dropping the borrow wrapper cannot undo refusal.
A successful drop, by contrast, leaves healthy inputs usable. Pending helper
cleanup stays reserved to the original manager instead of being replaced or
silently retried.

This establishes agreement among observations, not their authority. An NSS
snapshot's provider/configuration and helper namespace are not attested by
matching its numbers to a sequentially checked reader. Trusted host/initial
namespace, all-thread policy, authenticated original start, root-owned fenced
outputs and watchdog recovery remain prerequisites for production composition.
There is no launch or live IPC route, and these safety checks imply no measured
allocation, CPU/op or throughput improvement.

The [local packet](../testing/performance/0.74/local-runs/diagnostic-namespace-worker-binding-20261010/manifest.json)
retains the 242-pass Linux baseline, missing-API check and twelve new passing
tests. Final coverage passes 254 with one pre-existing ignored, followed by
three twelve-test repetitions. Seed `0x7502026` exercises 256 canonically valid
UID/GID/group mapping changes per repetition; these and injected observation
failures are synthetic. Real owned NNP cat processes exercise original readers,
exit refusal, healthy wrapper drop, early public-constructor refusal, pending
helper reservation and independent concurrency, using synthetic account
projections. A reader-level test verifies that binding refusal reaches both
privately owned components. Actual NSS operator inspection still refuses with
exit 9 and confirmed cleanup, so none of these positives become trusted NSS
enrollment. Windows passes 28 portable tests, and root contracts/evidence/
governance pass 127. Scoped lint/check and documentation gates pass; qualification
inputs remain exact and require-ship remains expected red.

## Worker identity authority before production preparation

Agreement between account and namespace-checked credentials does not decide who
is allowed to authorize the worker. The [authority audit](../testing/performance/0.74/diagnostic-host-policy-audit.md)
finds that provisioning v1 records installation/source identity, while the host
observation records host characteristics and backing-mount identity. Neither
enrolls the fixed account's UID, primary GID, complete groups, NSS provider or
helper namespace. The sysusers digest identifies an allocation request, not the
resulting mapping; the installed supervisor is root, distinct from the requested
unprivileged worker account. Backing-mount equality deliberately ignores the
namespace-local mount ID and cannot establish the helper's namespace.

GNU libc's [NSS design](https://raw.githubusercontent.com/bminor/glibc/glibc-2.39/manual/nss.texi)
separates lookup services and per-database configuration. Our resulting design
conclusion is that a digest of nsswitch.conf alone would not authenticate the
providers, data and execution context. Repeating the same lookup or adding
another consistency bracket would not close this authority gap.

The proposed narrow alternative is an explicitly approved local enrollment for
the fixed diagnostic worker: canonical nonroot UID/GID/groups under an external
policy pin and admitted host/boot context, followed by bounded inspection of
fixed local account files. General NSS remains a separate consistency diagnostic;
system-wide NSS need not change. The user approved this authority model for local
implementation; that decision does not enroll a real issuer or host. Kernel
namespace/context enforcement, durable reboot/revocation handling, all-thread
requirements and production integration remain separate. Local parser and reader
fixtures cannot become real host admission. No allocation/throughput gain follows
from these safety checks.

Two regression tests protect the current v1 boundaries: added worker-authority
fields must refuse, including canonical host-observation bytes with a matching
digest. These tests exercise existing strict decoding, not a new authenticated
policy. Production output preparation, signed diagnostic start and qualification
remain closed until their separate authority and lifecycle gates are complete.

## Signed local worker policy under external pins

The [first policy verifier](../testing/performance/0.74/diagnostic-worker-policy-local-design.md)
accepts only a separate diagnostic policy/envelope schema, never an extension
of provisioning or host-observation v1. Sorted canonical JSON with one LF,
4096-byte body and 8192-byte envelope ceilings makes the signed representation
explicit. A distinct signature domain and strict Ed25519 verification bind it
to an issuer key supplied outside the document. The externally supplied body
digest and current policy epoch are separate admission pins; no embedded key
or signature can select its own trust root.

The policy fixes the diagnostic-only local account source, account/group names,
nonroot UID/GID, exact sorted supplementary groups and complete passwd/group
document digests. Its group union with primary GID fits the existing 32-member
ceiling. Host/boot and user/mount namespace values must match explicit caller
assertions. This comparison authenticates no live host: fixed-file inspection
and kernel context enforcement remain separate work.

The runtime guard retains original policy, trust, context and envelope digest.
Changing an external key/pin/epoch refuses rather than refreshing the guard;
context or envelope drift and every other revalidation failure are sticky.
Reverification of the same valid policy is idempotent, not launch authorization.
The epoch is equality to an external current pin, not a durable monotonic ledger;
a fresh guard given stale operator configuration cannot detect that rollback.
Neither a policy extractor nor a conversion into a process/start capability is
exposed. No signer CLI, account mutation, NSS lookup or production route is added.

The [retained local packet](../testing/performance/0.74/local-runs/diagnostic-worker-policy-20261010/manifest.json)
records a 30-pass Windows baseline and missing-module refusal before implementation.
Twelve initial cases pass; reviewed coverage has thirteen, including oversized
revalidation and key/pin revocation. Final scope passes 43 Windows and 269 Linux
tests, with one pre-existing system-bus case ignored. All thirteen new tests pass
three further Linux repetitions. Seed `0x7512026` drives 256 re-signed valid
mapping/file-digest changes per repetition, first accepted under their own pins
then refused under the original pin. Signatures and host context remain synthetic.
Root gates pass 128 checks; scoped lint/check/format and documentation gates pass.
The frozen qualification digest and closed release admission remain unchanged.
These are policy safety results, not cache allocation or throughput evidence.

## Fixed local account files bound to the original policy

The [Linux account-file reader](../testing/performance/0.74/diagnostic-worker-files-local-design.md)
connects signed expectations to read-only file observations. It revalidates the
original policy under current external trust and asserted context before touching
files. Fixed names are opened relative to retained safe directory descriptors;
root ownership, no symlinks, no group/world writes, regular single-link leaves
and bounded sizes are required. A temporary fixture has a distinct type and
caller-owned origin, with no conversion to production proof.

Both complete account documents are hash-bound, including unrelated users and
groups. A conservative ASCII local-file parser rejects malformed or ambiguous
names, numeric aliases, noncanonical IDs and member lists. Worker UID/primary GID
must match the fixed user/group; explicit group membership must match the exact
signed supplementary list. The parser does not run NSS or infer an extra primary
group. Account maintenance therefore requires a new approved policy when these
documents change, even if the diagnostic worker itself is unchanged.

Two sequential rounds check original names, descriptors and security/size/time
metadata around bounded positional reads and mapping checks. Short reads refuse
rather than retry: [Rust's positional-read contract](https://doc.rust-lang.org/std/os/unix/fs/trait.FileExt.html#tymethod.read_at)
allows them. Errors permanently refuse the borrowed original policy, including
after dropping the file wrapper or restoring original files. The reader exports
neither file descriptors nor a launch capability.

These rounds are point-in-time consistency, not an atomic cross-file transaction
or privileged writer revocation. A changed-and-restored object indistinguishable
at every observation is outside this proof. NONBLOCK prevents waiting for FIFO
data but does not establish a regular-storage IO deadline. Actual host/boot and
user/mount namespace enforcement, real issuer enrollment, durable epoch/refusal,
all-thread credentials and authenticated original start remain prerequisites.
No account mutation, new helper/IPC route, workload or throughput claim follows.

The [retained file-reader packet](../testing/performance/0.74/local-runs/diagnostic-worker-files-20261011/manifest.json)
records 283 passing Linux tests (one pre-existing system-bus test ignored), 43
portable Windows regression tests and 129 root contracts/evidence/governance
checks. All 13 new integration cases plus the short-read unit case pass three
additional Linux repetitions. Seed `0x7522026` checks 64 valid unrelated-file
mutations against matching versus original digest pins per repetition. Exact
byte/line/record/group bounds pass; malformed, ambiguous, substituted, unsafe and
over-budget fixtures refuse. Check, strict lint, format and documentation pass.

Three negative findings are retained rather than folded into green summaries:
the missing-API diagnostic hit a compiler-renderer ICE and was reproduced as
ordinary E0432 with short diagnostics; a same-length metadata-mutation test was
changed to a deterministic length change after failure; governance required
explicit Linux-target registration. No gate or qualification threshold was
weakened. These findings improve test reproducibility and enrollment coverage,
not product allocations or throughput. Fixed production inspection and actual
admitted-context enforcement are still not established by temporary fixtures.

## Observed Linux context compared with signed policy pins

The [context guard](../testing/performance/0.74/diagnostic-worker-context-local-design.md)
tests a narrower safety hypothesis than production host enrollment: a valid
signature and matching caller assertions must still refuse when actual identity
documents or namespaces differ. The new reader compares `/etc/machine-id` and
procfs boot ID with the original signed values. It validates user and mount
handles separately as nsfs objects of the expected kernel types; a mount
namespace cannot substitute for a user namespace even with valid signed bytes.
Issuer key, body digest and epoch remain external expectations. The observation
API has no mechanism for choosing new trusted pins from what it finds.

Fixed names are opened read-only through retained descriptors with safe root
ownership, no symlink traversal except deliberate proc magic links, and explicit
filesystem checks. Machine and boot documents have exact 33/37-byte bounds,
including LF. Short, interrupted, extra-byte or changed-metadata reads refuse
without retry. Namespace checks bracket document opening and two later read
rounds; freshly resolved names and retained objects must still agree. The reader
neither exports those objects nor upgrades an old numeric/account guard.

Reading-thread identity matters independently of namespace equality. The guard
records the original thread ID and is neither Send nor Sync: a different thread
cannot inherit the observation simply because it currently shares namespaces.
Failure permanently refuses the borrowed original policy, including after the
wrapper is dropped or original descriptors are restored. External trust changes
are checked first, so a broken observation cannot hide revocation behind an IO
error. Successful drop leaves a healthy original policy available.

Actual local Linux positives use synthetic test issuer pins, not enrolled
operator keys. The result is sequential context consistency, not physical-host
attestation, an initial-host namespace proof, all-thread authority or a continuous
snapshot. Root ownership is interpreted inside the observed user namespace;
device/inode expectations require the externally approved boot. Regular-file IO
has no deadline proof and privileged writer revocation is unchanged. Durable
epoch/refusal, account-file/credential composition and authenticated original
start remain separate prerequisites. This safety change makes no cache allocation
or throughput claim and opens no production preparation, workload or ship route.

The [retained context packet](../testing/performance/0.74/local-runs/diagnostic-worker-context-20261011/manifest.json)
records 295 passing Linux tests with one existing system-bus case ignored, 43
Windows regressions and 130 root contracts/evidence/governance checks. Five new
integration and seven unit tests pass three additional repetitions. Seed
`0x7542026` exercises 64 well-formed re-signed foreign namespace values per run;
the new live comparison refuses every one after policy-byte verification accepts
them. Wrong namespace types, changed original descriptors/thread, short reads,
metadata growth and restoration are also covered. Check, strict lint, format
and documentation pass; the qualification pin and closed ship gate are unchanged.
The hypothesis is supported at this local safety boundary, not at production
enrollment. The next step must compose context with account-file opening and
kernel credentials; independent readers used in sequence cannot supply that proof.

## Account opening inside the original observed context

Two successful independent checks do not establish where account files were
opened: dropping a context reader and then creating an account reader loses
that relationship. The [new composition](../testing/performance/0.74/diagnostic-context-files-local-design.md)
tests the narrower hypothesis that the original observed context must bracket
account opening and every later account observation. No existing file guard can
be imported, and observations cannot choose new trust pins.

The constructor owns the original context guard, checks it, opens account files,
checks context again, reads the account mapping, then checks context once more.
Later revalidation is context/files/context without adopting replacement objects.
The private FileSet implementation is reused: descriptor safety, whole-document
hashes, strict local mapping, two account-read rounds and fixed budgets do not
change. Fixed root-owned input and explicitly owned temporary fixtures have
separate public types; neither exposes descriptors, mapping or start authority.

The first typed error ends the sequence. Context drift, failed opening, changed
file contents or names, revoked policy and bad mapping all refuse the original
policy. Restoring files or dropping the wrapper cannot reset it. Both origin
wrappers remain neither Send nor Sync, preserving the original reading-thread
constraint. This avoids transferring an observation to another thread; it does
not attest every worker thread or the kernel credentials that opened the files.

The [retained packet](../testing/performance/0.74/local-runs/diagnostic-context-files-20261011/manifest.json)
records 308 passing Linux tests (one existing system-bus test ignored), 43 Windows
regressions and 131 root contract/evidence/governance tests. Nine new integration
and four unit cases pass three additional repetitions. Seed `0x7552026` tests
64 valid re-signed foreign namespace values per run; byte verification accepts
the synthetic policy, actual context comparison refuses it before account access.
Tests cover every sequence failure position, first-error preservation, constructor
IO/mapping failures, replacement/restoration, revocation precedence and independent
concurrent readers. Scoped check/lint, format, docs and the new Linux root guard
pass. Require-ship remains expected-red and frozen qualification inputs stay exact.

This supports sequential context/account opening consistency only. Positives use
actual unprivileged WSL context with owned temporary account files under synthetic
issuer pins, not enrolled operator keys or a valid production worker account.
No atomic snapshot, continuous writer revocation, regular-file IO deadline,
initial-host proof, durable epoch/refusal or performance improvement is claimed.
The next slice is joining this owned context/account reader to namespace-checked
kernel credentials without refreshing any original object or clearing a refusal.
Production preparation, authenticated original start and qualification remain closed.

## Exact signed account mapping and retained kernel credentials

A signed local mapping and a healthy credential reader can disagree even when
each is internally valid. The [new binding](../testing/performance/0.74/diagnostic-policy-credentials-local-design.md)
compares the retained credential assertions with the original signed UID, primary
GID and supplementary list. It does not merge the primary group into the list:
primary membership and supplementary membership remain distinct signed decisions.
The adapter returns only a private boolean; no new numeric policy or projection
is allocated or exported.

Prior refusal or mismatch stops before IO. Otherwise the sequence is original
context/account, namespace-checked credentials, original context/account. Those
stages preserve the existing context/files/context and namespace/credentials/
namespace brackets, bounded parsers and process generation checks. First failure
preserves its typed error and refuses the original signed policy, account/context
reader and both privately owned credential components. Binding drop, restoration
and successful observation of another independent reader cannot clear refusal.
Healthy drop preserves the original inputs. Fixed and owned-fixture bindings stay
separate and inherit the original context's negative Send/Sync properties.

The [retained packet](../testing/performance/0.74/local-runs/diagnostic-policy-credentials-20261011/manifest.json)
records 324 Linux passes with one existing system-bus test ignored, 43 Windows
regressions and 132 root contract/evidence/governance passes. Fifteen binding tests
and one component test pass three additional repetitions. Seed `0x7562026`
varies 256 modeled mapping mutations per run. Real owned unprivileged NNP cat
helpers cover healthy revalidation/drop, original exit, constructor refusal,
revocation precedence, file replacement/restoration and independent concurrency.
Private gate tests cover every outer failure position and no IO after prior
refusal/mismatch. The first full Linux capture retains an unused-mut warning in
a new test; the annotation is removed and final tests/check/strict lint pass.
No product hypothesis or threshold changed.

This establishes sequential mapping consistency, not production enrollment.
Positives use actual local context and owned synthetic account documents under
test issuer pins. The binding accepts the original already namespace-checked
credential reader: its previous namespace bracket is retained, not upgraded into
attestation that status was opened under signed mount/host context. Worker mount
namespace, kernel file-opener credentials, all threads, initial-host authority,
atomic/continuous proof and durable epoch/refusal remain separate. No workload
or performance claim follows these safety results. Next: enforce signed-context
bracketing when opening status, while preserving original process identity;
real issuer enrollment and authenticated lifecycle activation remain closed.

## Opening status inside the original signed observer context

The previous binding compares a signed account mapping with a retained credential
reader, but cannot retroactively prove its opening context. The next
[isolated slice](../testing/performance/0.74/diagnostic-signed-opening-local-design.md)
therefore creates a fresh private reader from the original process under the
original signed account/context checks. It accepts neither preopened credentials
nor caller-supplied UID/GID/groups. Numbers come only from the checked original
policy; supplementary groups stay an exact list, not a primary-GID membership union.

The old namespace constructor opens status and immediately reads credentials.
Wrapping it would put the first observation before the post-open signed-context
check. Its existing behavior is preserved instead. A new crate-private opener
brackets opening with original namespace/generation checks and leaves the
projection unset. The outer sequence is account, open, account, credentials,
account: only the fourth stage observes credentials. Later checks are account,
credentials, account, without reopening. Account stages retain context/files/context;
credential stages retain namespace/credentials/namespace and bounded status parsing.

The first typed error stops and refuses the original policy/account and any owned
namespace/credential components. Restoration, dropping a failed reader or observing
an independent healthy reader cannot reset refusal. Healthy drop preserves borrowed
inputs, and the original process is never signaled. Fixed and fixture wrappers
remain distinct, neither Send nor Sync, without descriptors, reset or production
capability. This is sequential consistency, not atomic or continuous authority.

The [packet](../testing/performance/0.74/local-runs/diagnostic-signed-opening-20261011/manifest.json)
retains 340 Linux passes (one existing system-bus ignored), 43 Windows regressions
and 133 root contract/evidence/governance passes. Fifteen new opening tests and
one private opener test pass three further runs. Seed `0x7572026` explores 256
modeled failure positions each run. Valid freshly signed fixtures with wrong UID,
primary GID or supplementary membership refuse against a real owned process.
Other cases cover revocation before process/file errors, original exit, unhardened
credentials, file restoration, healthy drop and independently constructed readers.

A deterministic private seam changes the real fixture account file just after
status opens; its account error precedes the projection stage. A separate component
test verifies that the opener leaves no projection and an unhardened process
refuses only at the later credential read. These are retained-object and modeled
ordering checks, not exhaustive kernel race evidence. The missing-API compilation
failure before implementation remains in evidence.

Construction clones at most 32 signed supplementary GIDs once, not on later
observations. That explicit diagnostic cost is not a product allocation improvement.
No workload, RSS/goodput claim, host mutation or qualification is involved. Frozen
qualification inputs remain exact and ship admission stays closed. The bracket
does not attest kernel file-opener credentials, worker mount namespace, all
threads, initial-host authority or continuous writer revocation. Real issuer/
durable epoch enrollment and authenticated original lifecycle start remain separate.

## Retaining the original leader mount namespace

The signed opening reader checks the observer mount context, but its credential
component previously compared only user namespaces. A separate
[mount guard](../testing/performance/0.74/diagnostic-mount-namespace-local-design.md)
now observes the original retained process leader and reading thread sharing a
mount namespace. This isolates the new observation before composing it with
signed policy; passing standalone consistency cannot authorize production.

The reader retains worker and observer namespace descriptors and the original
worker ns directory. It pins the observer TID and is neither Send nor Sync.
Original generation checks bracket two observations of current and retained
objects, named worker directory identity and freshly named worker mount. Each
namespace descriptor must be nsfs with the kernel mount type, and device/inode
must match the original. The type comes from
[NS_GET_NSTYPE](https://raw.githubusercontent.com/torvalds/linux/v6.1/fs/nsfs.c),
not the text printed by a proc magic link. Only fixed proc magic links are followed,
using read-only CLOEXEC descriptors.

The first typed error stops and latches refusal. Restoring descriptors or private
test pins cannot reset it. Exit refuses even when a retained namespace descriptor
survives. The original process is borrowed, never refreshed, signaled or exported;
healthy drop preserves it. Independent readers constructed on separate threads
keep separate refusal state. Three original directory/namespace FDs are retained;
transient lookup descriptors are released at each observation.

The [packet](../testing/performance/0.74/local-runs/diagnostic-mount-namespace-20261011/manifest.json)
records 356 Linux passes with one existing system-bus ignored, 43 Windows
regressions and 134 root contract/evidence/governance passes. Sixteen new tests
pass three additional repetitions. Seed `0x7582026` drives 256 modeled iterations
of failing stages and device/inode changes in the final suites. The first green
preceded the extra seeded identity cases; it remains an exact earlier capture.
Missing-module E0432 and its dependent E0282 before implementation are retained,
alongside expected-red ship admission.

Owned local helpers exercise real namespace descriptors, readonly flags, original
exit and wrong-type/substitution refusal. Observer TID, stored directory pin and
distinct mount identity mutations are private deterministic cases. No foreign
namespace or real setns/unshare transition is exercised. This does not prove
every worker task, initial-host namespace, mount contents remaining unchanged,
atomic/continuous inspection or kernel file-opener credentials. In particular,
mount-tree contents can change while namespace identity stays equal.

This stage measures safety, not cache allocations, CPU/op or throughput. Product
code and frozen qualification inputs remain unchanged. Next, the signed composition
must derive the mount guard's process from the original credential reader without
allowing a substitute argument. Bounded all-thread observations, durable issuer
enrollment and authenticated lifecycle still have independent requirements.

## Joining signed credentials with the same original leader mount

A healthy credential observation and a healthy mount observation are not enough
if callers can select two different processes. The new
[signed composition](../testing/performance/0.74/diagnostic-signed-mount-local-design.md)
borrows the original signed credential reader and privately opens the mount guard
from that reader's retained process. Its public binders accept no ProcessRead,
PID/path or already opened mount guard. An observed namespace never becomes a
new trusted assertion: original device/inode must match the original checked
signed policy.

Construction orders signed/open-mount/signed/mount/signed. Each signed stage
retains the original context/files/context and account/credentials/account checks,
including user-namespace observations. The inner mount opener preserves its own
generation checks; the outer mount stage follows the post-open signed check.
Subsequent observations use signed/mount/signed without reopening. First typed
failure stops, refuses the original signed reader, account/context/policy and both
credential components, and refuses any newly owned mount guard. Restoring files,
dropping the wrapper or supplying old pins cannot repair that original guard.
Healthy drop leaves the original inputs usable. Fixed and fixture origins stay
distinct and neither wrapper is Send nor Sync.

The [local packet](../testing/performance/0.74/local-runs/diagnostic-signed-mount-20261011/manifest.json)
records the missing-API failure before implementation, real owned helpers and
deterministic private boundary failures. Thirteen composition tests cover original
exit, post-open account failure before a bad mount, revocation precedence, mount
substitution/restoration, prior refusal, independent concurrent readers and sticky
refusal after drop. One component test verifies original-process derivation and
refusal of either inner credential guard. The final scoped suite passes 370 Linux
checks with one existing ignored, 43 portable Windows checks and 135 root
contract/evidence/governance checks. Parallel and serial repetitions each
pass the 13 tests three times before reviewed hardening; three more serial
repetitions pass afterward. The follow-up checks inner guard refusal before
account IO as well as mount opening, with each inner component independently
refused while the fixture account file is missing. The serial logs retain seed
`0x7592026` and 256 modeled failing positions without stdout/stderr interleaving.

Two inherited source-shape contracts and a new lint initially failed. The fix
preserves their assertions and public observation bodies; the failed captures
remain alongside the corrected checks. The first 12-test green is an earlier
observation, not a replacement for the later context-restoration regression.

This is safety evidence, not a cache speedup. Three retained mount FDs and repeated
signed/mount observations have diagnostic costs; none is reported as measured
CPU/op, allocation or goodput improvement. Leader equality still does not inspect
every task or freeze mount contents. Synthetic issuers and temporary fixtures
still do not prove production ancestry or durable revocation: a fresh guard under
restored external pins is outside in-memory sticky refusal.

The [production readiness audit](../testing/performance/0.74/production-readiness-and-server-launch-20261011.md)
therefore separates local implementation, controlled host enrollment and product
release. Bounded all-task checks, real issuer/durable epoch authority, fenced
production preparation, authenticated original diagnostic start, writer revocation
and watchdog/reconciliation must precede a host pilot. Exact C74/native/Redis and
long-run/supply-chain evidence must precede product release. A green local reader
cannot bridge either boundary by renaming a fixture or flipping an admission flag.

### Completing local gates exposed a build-supply-chain defect

The broader readiness request adds a different hypothesis: does a successful
local build conceal a skipped surface or a known dependency issue? The
[extended readiness packet](../testing/performance/0.74/local-runs/production-readiness-local-20261011/README.md)
retains the terminal Windows verifier exit 0. Strict lint/allocator, dependency,
workspace/xtask, doctest, rustdoc and budget gates pass. Its 3,982 passing and 82
ignored harness executions are not unique tests or a throughput measurement.
The run spanned development changes, so it is not an immutable exact-candidate
receipt or a replacement for the unchanged registered fast-gate time budget.

The console gate explicitly skips because native `npm` cannot be found. Running
the same install/build/test chain through npm.cmd covers the omitted surface:
16 unit, six package/static/supply-chain and 46 browser tests pass twice. Linux
supervisor compilation also passes on Rust 1.88 MSRV; scoped Linux rustdoc passes
with warnings denied. Neither result supplies positive fixed-host enrollment.

The first complete npm audit nevertheless reports two high-severity dev packages.
They belong to ESLint/Vite/coverage build and test paths, not a demonstrated runtime
cache exploit. The existing parent ranges admit `brace-expansion` 5.0.12 and
`source-map-js` 1.2.2. These address the reviewed
[brace parsing](https://github.com/advisories/GHSA-q2hr-2g5m-vwhr) and
[indexed source-map offset](https://github.com/advisories/GHSA-68fv-2mgg-jv7q)
DoS advisories. Two new locked-version regressions first fail on the old lockfile;
only the two transitive version/URL/integrity records are then updated. npm ci
and repeated full audit report zero vulnerabilities at capture time. No blanket
audit fix, dev omission, direct dependency drift or threshold waiver is used.

Review finds a second, test-only weakness: checking a healthy root copy could
miss an old nested copy. A fixture canary first reproduces that false positive,
then the guard enumerates all matching lock paths and refuses missing packages.
The reviewed unit/package chain passes 16 and seven tests. The earlier browser
runs remain evidence for unchanged UI fixtures, not a new host lifecycle result.

Vite also preserves Windows source HTML line endings. Pinning that input to LF
and rebuilding removes incidental generated/embedded Git content differences;
no generated asset blob is committed. Same-host deterministic bundle tests pass,
but cross-host reproducibility and final source-bound packages remain separate.
The retained 182-component console SBOM still describes local version-0.73 tooling,
not an accepted 0.74 product candidate. This work fixes a confirmed supply-chain
finding and strengthens regression coverage; it establishes no CPU, allocation,
memory or goodput gain. The remaining server and product boundaries are listed in
the [launch audit](../testing/performance/0.74/production-readiness-and-server-launch-20261011.md).

### Observing every worker task requires retained membership

Leader credentials do not establish the credentials or namespaces of another
thread. The [original-task guard](../testing/performance/0.74/diagnostic-task-set-local-design.md)
tests this boundary separately from product optimization. It pins a bounded
inventory of at most 128 tasks through the original process descriptor, retains
each task's generation and status/namespace objects, and compares two complete
passes under process-generation and membership brackets. Additions, removals,
descriptor replacement or policy drift refuse instead of refreshing the inventory.

Real owned unprivileged Python helpers cover one and several threads, additions,
removals, process exit and independent concurrent readers. A parallel suite exposes
a fixture synchronization problem: the removal check refuses, but does not always
return the expected inventory `Drift` error. The original failure lacks the actual
variant, so its exact cause is not established. The helper now acknowledges removal
only after the task is absent from procfs, not merely after Python join. This is a
bounded logical fixture barrier; the observer still performs no retry. Directory
grammar, all UID/GID slots, TGID, groups, active capabilities, NoNewPrivs, bounding
set drift and namespace substitution also have focused regression tests.

The [packet](../testing/performance/0.74/local-runs/diagnostic-task-set-20261011/README.md)
retains the baseline, absent-API red result, initial fixture failures and reviewed
checks. Modeled first-error schedules and directory mutations are not live kernel
race counts. Stable sequential scans can miss tasks that appear and disappear
entirely between observations, and shared namespace identity does not attest an
initial host namespace or immutable mount contents. This primitive proves local
consistency against numeric assertions, not original signed enrollment. The next
step is a private original-signed-reader composition with refusal propagation;
durable issuer authority and authenticated production lifecycle remain separate.
No throughput, allocation, CPU or memory improvement follows from these safety
checks, and frozen qualification inputs are unchanged.

### Binding task observations to the original signed reader

The next hypothesis is about attribution integrity, not speed: can task checks
be joined to the already verified worker without letting a caller replace the
process or numeric policy? The
[signed-task composition](../testing/performance/0.74/diagnostic-signed-tasks-local-design.md)
answers this locally with an exclusive borrow of the original signed mount
reader. The process is derived privately through the original credential guard;
task UID/GID and the exact supplementary group list come from its original signed
policy. Both retained user and mount namespace pins must match that policy.
Separate fixed and fixture types cannot be converted or transferred across threads.

Opening brackets bounded task observation with signed checks; later observation
uses signed/tasks/signed. The task primitive itself retains its generation and
membership brackets. A failure stops at the first typed error and latches the
entire account/context/policy/leader-credential/mount/task chain. Dropping the
wrapper cannot recover it, while a healthy drop leaves the original reader usable.
Previously refused inner components are tested before account IO, rather than
being allowed to reach a different error through damaged signed bytes.

Tests exercise real owned nonroot single/multi-thread helpers, additions/removals,
exit without replacement adoption, independent readers, signed account restoration,
post-open signed failure and private descriptor/policy/namespace substitutions.
Seeded modeled schedules check exact order and first-error/no-retry semantics;
these are not kernel-race or performance samples. An initial API-path mistake and
a strict test lint finding are retained separately from the absent-API expected
red and successful reviewed runs. Neither requires weakening a safety predicate.

This closes a local composition gap, not production enrollment. The bounded scans
can still miss a transient task entirely between observations. No real foreign
namespace or nonleader credential mutation is demonstrated; namespace identity
does not freeze mount contents or prove initial-host authority. Operator issuer
pins, durable revocation and rollback scope, authenticated original start, watchdog
and exact writer/cgroup cleanup remain separate prerequisites. The change makes
future attribution harder to misbind; it does not establish a throughput, CPU,
allocation or memory improvement. Product candidates and ship admission stay closed.

The [retained packet](../testing/performance/0.74/local-runs/diagnostic-signed-tasks-20261011/README.md)
binds the reviewed implementation `f624df7f9c9da31f18f0ee2841506253df382bc6` and 41
raw logs. Eleven new unit tests pass in three serial repetitions; reviewed scoped
Linux checks pass 394 tests with one existing ignored, Windows passes 189 portable
tests, and root contract/evidence/governance checks pass 101/13/23. Strict clippy,
all-target checks and MSRV 1.88 pass on both platforms. These counts describe test
executions and local safety coverage, not unique performance samples or a new
full-workspace/exact-candidate release receipt.

### A refusal must have a durable owner, not just an in-memory guard

The signed worker checks already refuse their original objects permanently, but
that permanence ends with the process. A new guard given restored old operator
pins can otherwise repeat the old authority decision. The next local hypothesis
is therefore about trustworthy attribution: can a bounded journal preserve an
explicit refusal and prevent same-epoch reactivation across reopening?

The [preregistered design](../testing/performance/0.74/diagnostic-authority-registry-local-design.md)
selects local durable storage as the target. It deliberately excludes whole-host
snapshot rollback and owner/root tampering; detecting those needs an independent
monotonic authority. No actual issuer is selected here. A canonical v1 record
binds issuer, policy digest, epoch, machine, boot and both namespace assertions.
Refusal retains that exact binding. A new active record requires a refused
predecessor, a strictly higher epoch, the same issuer/machine and a new policy
digest. Revision and predecessor digests make gaps and changed refusal bindings
invalid; wall-clock time has no role.

The first storage implementation is a separate nonroot Linux /tmp fixture, not
production-root enrollment. An exclusive nonblocking directory lock prevents
cooperating concurrent writers. Retained directory and file identities detect
observed replacement or metadata drift. Each generation uses two exclusive
create-new files: canonical record and digest marker. File and directory fsync
order publication; no previous file is overwritten, renamed, deleted, repaired
or evicted. Bounds are 128 generations, 2048 record bytes, 65 marker bytes and
256 directory entries. Exhaustion refuses rather than rewriting the history.

The failure cases matter more than the happy path. Real local close/reopen tests
retain an explicit refusal and accept a later epoch only with newly supplied
independent pins. Private logical faults after record creation/sync, marker
creation and final sync leave the original writer refused. Incomplete pairs
refuse reopening. A late error can leave a complete refused generation visible
without an acknowledged success: inspecting that state is correct; assuming
rollback or replaying the write is not. Partial writes followed by ENOSPC exercise
the shared write helper. Symlinks, hard links, FIFO, unsafe mode/owner, excessive
bytes/entries, stale pins and original descriptor/path replacement are tested.
Seeded history mutations are grammar/transition models, not kernel-race counts.

This evidence supports the bounded transition and local fsync protocol, not
physical power-loss behavior, hardware guarantees or an IO deadline.
The predecessor hash chain is not historical signature attestation and cannot
defend against an owner rewriting the entire journal. A crucial remaining gap is
also explicit: only successful explicit refusal is persisted. Inspection or IO
failure latches the current writer in memory, without trying to append to unsafe
storage. Original signed-guard durable failure propagation and refusal on
uncertain lifecycle restart still need their own composition.

Windows strict lint initially finds Linux-only creation helpers compiled as dead
code in the portable library. Platform gating fixes the separation; no warning
or safety check is suppressed. The
[retained packet](../testing/performance/0.74/local-runs/diagnostic-authority-registry-20261011/README.md)
keeps the absent-API red, initial lint finding and successful reviewed local
checks separately. This step improves the reliability of future attribution
evidence, not cache throughput, allocations, CPU or retained memory. The next
implementation boundary is root-owned fixed storage and its exclusive join to
the original signed guard; real issuer enrollment and production start remain
closed, as do product acceptance and release qualification.

Parallel review then exposes a second finding: a complete late-failure refusal
occasionally cannot reopen because flock remains held. A deterministic retained
descriptor duplicate reproduces the same lifetime problem without a fork. The
owner now explicitly unlocks on drop; closing the old duplicate afterward cannot
release the new writer's independent lock. The original parallel alias is not
identified, so this establishes a concrete lifetime weakness rather than claiming
an observed fork trace. Both negatives and the reviewed fix are retained.

A stricter frozen-input audit finds a separate inherited release blocker. The
plan changed at `8e16117b`, but the qualification manifest still pins the earlier
plan digest. LF normalization does not remove this content mismatch; Windows
CRLF adds a separate raw-byte difference. The qualification dry-run correctly
refuses even though the structural local contract passes. This slice leaves both
frozen files unchanged and reports the mismatch instead of refreshing a digest
to make the guard pass. Future qualification requires an explicit reviewed
reconciliation of the plan and its identity.

Reviewed local checks pass 409 scoped Linux tests with one existing ignored,
195 portable Windows tests and 102/13/23 root contract/evidence/governance tests.
The fourteen new unit tests repeat three times serially. Strict lint, affected
all-target/MSRV checks, formatting, docs and book checks pass. These are local
safety executions; neither the inherited qualification refusal nor any production
gate is converted into acceptance by these green results.

### A fixed pathname must retain its original ancestry

The next local hypothesis separates storage mechanics from storage origin. The
temporary journal already exercises canonical bytes, bounded transitions and
publication faults. Copying that implementation for production would create two
protocols to keep consistent. Instead, a distinct fixed-path writer reuses the
same private engine and cannot be obtained by converting a fixture writer.

The [preregistered scope](../testing/performance/0.74/diagnostic-authority-fixed-local-design.md)
fixes `/var/lib/hydracache-performance/worker-authority-074`. Public constructors
accept checked policy or independent trust/context, not caller-selected paths,
owners or descriptors. Effective root credentials are a preliminary ownership
gate, not proof of an enrolled initial host namespace. Existing directories must
already be provisioned; this implementation does not change the installer.

The important change is descriptor ownership across the entire pathname. The
writer opens each fixed component without following symlinks, retains every
original object and compares named and retained identities around journal IO.
Root:root ancestors must not be group/other writable or carry special mode bits;
the leaf is exactly 0700. Directory timestamp/link-count changes are intentionally
not identity changes: an unrelated sibling must not spuriously revoke authority.
Observed mode, ownership, named-object or descriptor replacement does revoke it.

Local tests use the actual bounded walker under nonroot temporary modeled roots
and synthetic stamps for the root ownership predicate. They cover each ancestor,
the leaf, a retained parent alias, symlinks, missing/unsafe components, contention,
reopening and all four publication fault boundaries. The absent exported API is
retained as test-first red evidence. These tests establish the local API and IO
behavior; they are not positive production root-path enrollment or physical
power-loss tests. Cross-filesystem ancestry is allowed, while record files must
stay on the leaf filesystem; approved mount/context binding remains separate.

The [evidence packet](../testing/performance/0.74/local-runs/diagnostic-authority-fixed-20261011/README.md)
keeps local repeat checks distinct from release admission. The design pays off
as a single publication protocol with two non-interchangeable origins, not as a
numerical cache optimization. Successful explicit refusal is still the only
durable refusal operation: unsafe storage cannot safely be repaired by appending
an automatic failure record. The next implementation is the exclusive join to
the original signed worker guard and its uncertain-restart rules. Real issuer
enrollment, controlled fixed-host checks, watchdog/cleanup and production start
remain closed. The inherited qualification plan-digest mismatch is unchanged.

Review yields two additional findings. Holding the original root descriptor alone
can conceal named-root replacement in the local model. A failing regression now
requires comparison with a freshly opened named root as well; it passes after
the fix. This does not simulate a real host `pivot_root` or mount transition.
Another serial repetition exposes an older fixture test's timestamp assumption:
an immediate same-byte rewrite can occur within one filesystem clock tick. The
test now forces and asserts observable mtime drift. Runtime checks are unchanged;
no sleep, retry, threshold relaxation or transient-owner-tampering claim is added.
Both red results remain in the packet.

The final local packet retains 51 raw logs: 419 scoped Linux passes with one
existing ignored, 195 portable Windows passes and 103/13/23 root contract,
evidence and governance passes. Twenty-three journal unit tests, including nine
new fixed-backend tests, pass in three separate serial repetitions. Strict lint,
MSRV 1.88, Linux rustdoc, formatting, local contracts, docs and book checks pass.
These counts describe local safety tests, not cache performance samples or
exact-candidate release receipts; no throughput or allocation claim is added.

### A signed check must refuse the same original authority

The next [preregistered bridge](../testing/performance/0.74/diagnostic-signed-authority-local-design.md)
tests a narrower question than production launch: can failure of the original
signed task guard reach its original durable journal without accepting a new
policy, process or storage origin? The wrapper holds both objects exclusively.
Fixed and fixture constructors remain distinct, and the only saved pins come
from the already checked original policy, not the current caller's assertions.

Every successful observation is bracketed by journal / signed tasks / journal.
A signed failure first refuses the entire original guard chain. A fresh safe
inspection under the original pins then permits one append-only refusal. A
journal already containing refusal stops before the signed step and receives
no extra record. Foreign bindings, advanced epochs and unsafe storage refuse
continuation without writes or repair. When publication fails, the error retains
both the first failure and the unacknowledged persistence result. Later calls
return immediately; drop does not hide a retry.

This is useful safety composition, not a measured cache optimization. The local
tests use real owned helper processes and proc/nsfs observations, synthetic
signatures and temporary account/journal fixtures. They cover healthy drop,
policy/context/account drift, task churn, explicit refusal and reopen, stale
epochs, concurrency and all four journal publication-stage faults. Fixed typed
dispatch is modeled privately; it does not enroll a root-owned production host.

The most important negative result is intentional: a bridge that cannot safely
write to changed authority leaves that other journal intact and potentially
active on reopen. Calling an in-memory latch durable would conceal this gap.
Partial publication tails already refuse reopen; a complete but unacknowledged
refusal reopens refused. A crash before any append is a different problem and
still requires a separately reviewed lifecycle intent protocol, watchdog and
cleanup. No production start/restart API, real issuer, qualification permission
or new record format is introduced. The inherited qualification digest mismatch
is unchanged.

The [local packet](../testing/performance/0.74/local-runs/diagnostic-signed-authority-20261011/README.md)
keeps failed attempts alongside reviewed checks. The missing API first fails;
the initial callback form fails Rust lifetime checking; strict lint catches an
unnecessary borrow. A foreign-context test initially signs an invalid boot ID;
the fixture is corrected to a valid distinct UUID, without weakening validation.
An initial baseline filter selects zero tests and is not counted as a pass:
the unchanged prior test binary subsequently runs all eleven signed-task tests.

The reviewed checks pass 437 scoped Linux tests with one existing ignored,
195 Windows tests and 104/13/23 root contract/evidence/governance tests. Seventeen
new unit tests pass three separate serial repetitions; two compile-fail doctests
confirm no clone or fixture promotion. Strict lint, MSRV 1.88, rustdoc, formatting,
local contracts, docs and book checks pass. These are local safety results, not
full-workspace release receipts. Unsupported command options are retained as
usage failures and are followed by separate corrected invocations; actual ship
admission and the inherited qualification dry-run still refuse.

### Interrupted diagnostic intent must not become a second start

The next [local recovery check](../testing/performance/0.74/diagnostic-fixture-recovery-local-design.md)
tests whether an interrupted diagnostic can refuse the same original signed
authority without repairing or replaying its lifecycle. The existing coordinator
already publishes Starting before calling its backend, and the IPC model writes
an intent before mutation and acknowledges it afterward. Introducing another
journal would duplicate these local boundaries. This slice instead reuses their
existing canonical decoders and the authority journal's existing Policy refusal.
No persisted format or epoch transition changes.

The fixture check retains the original cooperative host lock through inspection
and refusal handling. It reads only the two fixed journal names relative to the
retained root, with no-follow/nonblocking opens, bounded reads and exact private
metadata checks. A second observation checks original objects, named objects,
bytes and previously absent files. Pending names are detected without following
them, including dangling symlinks. Corrupt or future-schema documents cannot
become a quiet result merely because their hashes were rewritten.

Pending publication, an IPC entry without an acknowledgement, and Starting,
Running or Stopping refuse recovery. Missing state, wrong identity/boot, a
competing lock or unsafe metadata fail explicitly. The bridge first checks its
original signed guard and authority under retained original pins. On a lifecycle
failure it latches the whole chain, then appends only to a freshly inspected,
safe matching authority journal. A failed append preserves both the original
failure and publication uncertainty; later calls neither retry nor inspect
replacement files. Lifecycle documents and output remain retained unchanged.

The hypothesis pays off as bounded safety composition, not cache throughput or
allocation reduction. Reserved and Terminal yield observation data only: neither
authorizes starting a worker, adopting a live process or removing a marker. A
compile-fail test excludes the fixture check from the fixed-origin API. Real
issuer enrollment, authenticated production intent, reboot handling, watchdog
and owned-tree cleanup remain separate work. A crash before any durable intent
exists is still outside this proof.

The [packet](../testing/performance/0.74/local-runs/diagnostic-fixture-recovery-20261011/README.md)
retains the missing-API red and all implementation attempts. Initial fixture
ledger creation used the default 0644 mode; the required private 0600 check
correctly refused it. Correcting the fixture restores the expected results
without relaxing metadata validation. A structural contract check also initially
mistook rustfmt's line break before `read_at` for a missing positional read; its
marker now matches the complete call independently of that line break. These
findings are test-construction errors, not measured product regressions.

The completed packet retains 37 raw captures: 494 scoped Linux passes with one
existing ignored, 195 portable Windows passes, and 105/13/23 root contract,
evidence and governance passes. Fourteen new unit tests pass three serial
repetitions alongside two existing recovery tests. Three compile-fail doctests,
strict lint, MSRV 1.88, rustdoc, formatting, docs, local contracts and book checks
pass. Final root checks use native Windows: Linux Git cannot parse the Windows
worktree pointer, and a global metadata override would be unsafe for tests that
create temporary repositories. Failed/interrupted attempts are retained and not
counted as successful proof. Qualification dry-run and ship admission remain red;
no cache workload or numerical performance claim accompanies this safety slice.
### A signed configuration is not an authenticated running worker

The [next local contract](../testing/performance/0.74/diagnostic-intent-projection-local-design.md)
tests a narrower hypothesis: can a controller's signed intent be tied to the
same original worker policy and exact reserved cell without introducing another
start path? The byte verifier uses a separate signature domain and strict
canonical JSON. It checks independent controller pins and principal allowlists,
then binds the full state digest, original policy digest/epoch, boot and a digest
of the existing fixed unit builder. Typed properties and ordered arrays preserve
differences in environment, argv, limits, output paths and command flags. A
different numeric type must not hash as the same property.

The checked intent exclusively borrows the original worker policy. Revalidation
cannot adopt another cell, controller configuration, peer or re-signed replacement
envelope. An expired or drifted checked intent stays refused after restoration.
A worker-policy refusal remains on the original worker after the wrapper is
dropped. In contrast, a malformed incoming controller packet does not revoke a
healthy unrelated policy: rejecting a request and revoking worker authority are
different decisions. Neither operation starts, stops or writes anything here.

This hypothesis is supported as a local byte-binding prerequisite. The
[retained packet](../testing/performance/0.74/local-runs/diagnostic-intent-projection-20261011/README.md)
contains 16 new integration tests, two unit tests and three repeated integration
runs, including 64 fixed-seed state mutations, all four cells and signature,
schema, budget, deadline, boot and trust failures. Linux scoped tests pass
451 cases with one existing ignored; portable Windows tests pass 195. Five
compile-fail doctests, strict lint, MSRV 1.88 and local documentation/contract
checks pass. These totals describe the chosen local scopes, not unique release
coverage or an exact-candidate qualification receipt.

The initial parallel baseline retained an existing `Text file busy` failure;
the serial baseline passes 433 tests with one ignored, without changing that
test or its validator. Governance first refuses the missing cfg-gated target
registration, then an invalid registry field; the corrected entry uses the
existing Linux supervisor CI step. Both failed attempts remain visible. No
threshold, frozen plan input or admission rule was weakened to pass a check.

The important negative conclusion is unchanged: signature validity is not live
host attestation. Peer and time are asserted, state is not retained under a
production fence, and unit hashing says nothing about effective runtime or
original file descriptors. Identical requests can be checked independently;
there is deliberately no replay ledger or execution capability. Durable signed
intent, crash/reboot rules, original material/output/runtime composition and
controlled enrollment remain before production. No cache benchmark is run and
no allocation, goodput or latency improvement is claimed by this safety work.

### Durable intent consumption before execution

The next hypothesis concerns crash safety rather than throughput: retaining the
exact signed request before execution should prevent a lost acknowledgment from
turning a retry into another consumption of the same cell. The
[local journal](../testing/performance/0.74/diagnostic-intent-journal-local-design.md)
tests that boundary using owned temporary directories and synthetic controller
keys. It acquires the existing cooperative host fence, checks the exact persisted
Reserved state and refuses pending publication or conflicting lifecycle context.
Four fixed record/marker pairs retain the full state, signed envelope, asserted
peer and predecessor digest. Canonical byte and directory bounds reject malformed
or oversized history before appending another record.

Exclusive creation, file sealing and file/directory fsync make publication
observable as stages. Faults after creation, record sync, marker creation or final
commit leave the files in place and latch the original checked intent. An
incomplete pair remains uncertain, without automatic repair. A complete pair
whose acknowledgment was lost remains consumed: reopening gives inspection data,
not permission to retry. Historical signatures are authenticated at their signed
issue instant only to establish what was consumed; that check never renews an
expired request's eligibility. New signatures cannot bypass cell, nonce or request
deduplication within the same retained journal.

An inode-substitution regression exposed an important distinction between equal
bytes and the original object. Reopening a record by name after publication could
accept a replacement file containing identical bytes. The first test appeared
green only because an incorrectly permissive parent fixture refused before the
intended checkpoint. Requiring that the test actually reach RecordSynced, and
setting the fixture's preregistered 0700 permissions, produced the real failing
regression. The fix retains the originally created descriptor and sealed metadata,
then compares that object with the named leaf throughout publication. The same
rule applies to markers and earlier history records. Validators were not relaxed.

The [retained packet](../testing/performance/0.74/local-runs/diagnostic-intent-journal-20261011/README.md)
contains 19 new unit tests with three serial repeats, final selected Linux results
of 471 passes and one existing ignored, 195 portable Windows passes and six
compile-fail doctests. Strict lint, MSRV 1.88, documentation and local contracts
pass. The packet also preserves the ineffective green test, incomplete-test
compiler ICE, test compile error and fixture failures. Earlier projection evidence
stays bound to its earlier Git source; it is not relabeled as evidence for changed
code. These counts describe local scopes, not unique release-wide coverage.

The hypothesis is supported for conservative consumption in one fixture journal.
It does not prove physical power-loss behavior, resistance to owner/root tampering
or whole-host rollback, or actual once-only execution. Another fixture root has
independent history because the signed request does not authenticate a production
storage root. The next boundary is a fixed root-owned intent backend joined to
the original authority and material/output/runtime guards, followed by watchdog,
owned cleanup and controlled real issuer/host enrollment. Qualification still
refuses its inherited frozen plan-digest mismatch. No cache workload or numerical
allocation, latency, CPU or goodput claim accompanies this safety prerequisite.

### Fixed storage ownership before publishing intent

Durable fixture consumption does not establish that production storage is trusted.
The next audit found a concrete reason: existing tmpfiles provisioning assigns
the campaigns directory to `hydracache-perf:hydracache-perf` with mode 0750.
Its parent is root:hydracache-perf. A worker-owned directory may be appropriate
for campaign artifacts, but cannot simply become the root authority journal.
Changing its owner to make a test pass would change the deployed protocol.

The [next local prerequisite](../testing/performance/0.74/diagnostic-intent-fixed-storage-local-design.md)
therefore inspects a separate prospective fixed intent leaf without creating it.
The guard opens `/` and each fixed path component with no-follow directory
descriptors, requiring root ownership, no group/other write or special bits,
and root:root 0700 at the leaf. The existing performance parent's group is
permitted only under that nonwritable policy and is captured exactly. Rechecks
compare every original descriptor with its current named object, including `/`.
Normal directory timestamps and link-count growth are not identity changes;
owner, inode, device or exact mode changes refuse permanently.

Seven new local unit cases pass three repetitions. The selected Linux scope
passes 479 tests with one existing ignored; root checks pass 108 contract, 13
evidence, six gated-target and 23 governance cases. These modeled nonroot trees
exercise the actual private walker, not positive root storage enrollment. The
[packet](../testing/performance/0.74/local-runs/diagnostic-intent-fixed-storage-20261011/README.md)
retains baseline, absent-API failure and scoped checks. The public guard exposes
no configurable origin, descriptor, clone or execution conversion. It neither
publishes intent nor authenticates an issuer, live host, fence or journal records.

The audit also finds that the older fixed authority backend requires root:root
for the common parent, unlike the current provisioning bundle. The new inspector
does not quietly relax that older contract or claim the two can already compose
on the host. A reviewed common ownership policy, lease-specific retention and
original authority/fence/state join remain before a fixed publisher. Sequential
directory inspection is not continuous confinement or physical power-loss proof.
No server, cache workload, frozen qualification input or performance threshold
changes accompany this step; its benefit is an explicit storage trust boundary,
not measured throughput or allocation improvement.

### Preparing a storage check on the server

The subsequent [storage preflight](../testing/performance/0.74/diagnostic-storage-preflight-design.md)
resolves the ownership mismatch rather than changing deployed campaign ownership.
Only the common root-owned performance parent's group requirement is amended:
the provisioned GID is accepted when group/other write and special bits are
absent, then captured and compared exactly on every recheck. The remaining
ancestors and the two private 0700 leaves still require root:root. Changing the
group later is an identity change, not a harmless refresh. Journal formats,
issuer pins, epoch rules and cache product behavior are unaffected.

The signed provisioning input now includes both private leaves. A new read-only
guard runs before installation mutation and refuses unsafe existing ancestors,
symlinks or incorrectly owned/permitted leaves. This ordering matters because
tmpfiles can otherwise adjust an existing directory's metadata, hiding the very
condition the runtime should refuse. Safe existing leaf contents remain in place;
neither installer nor ancestry preflight authenticates their journal records.

The fixed Linux CLI opens independent original authority-storage and intent-storage
guards and rechecks authority, intent, authority and intent. It emits a bounded
JSON schema with one positive fact, `storage_ancestry_verified`; journal proof,
authority binding, host enrollment, execution, qualification and ship remain
false. Missing paths and nonroot execution refuse, rather than initializing
storage. There is no path selector, key enrollment, helper, shell, lock or worker
start hidden behind a successful check.

Local modeled tests exercise both leaves and every ancestry level with directory
replacement, symlinks, mode changes and synthetic GID-stamp drift. They verify
permanent refusal and tolerate unrelated directory growth. Separate order/failure
tests prove that a failed observation stops the bracket and produces no success
report. CLI and provisioning regressions cover the refusal and installation
boundaries. The [packet](../testing/performance/0.74/local-runs/diagnostic-storage-preflight-20261011/README.md)
retains baseline, failing tests and scoped checks; modeled GID drift does not
claim privileged chown coverage or positive fixed-host enrollment.

This prepares a controlled read-only server test after a fresh signed installation.
It is not the missing diagnostic worker pilot. The fixed intent publisher still
needs the original authority, fence and Reserved state, followed by production
output creation, start, watchdog and owned cleanup. Real issuer/host admission
also remains necessary. Historical evidence is checked against its original Git
source, not rewritten to match the amended code. The qualification plan-digest
refusal remains visible. No throughput or allocation gain is inferred from these
safety checks.

The root contract initially refused importing the output-preparation namespace
into the CLI. A separate public read-only facade exposes only storage inspection;
the original no-preparation route check remains intact. Final root checks pass
109 performance contracts, 13 evidence, six gated-target and 23 governance tests.
The selected Linux scope passes 451 tests with one existing ignored, the portable
Windows run passes 196, and eight compile-fail doctests pass. The four new
preflight cases have three separately retained repeats. These are scoped local
results, not exact-candidate qualification or unique release-wide test counts.

### Composing signed intent with the original authority

The next safety dependency is ownership of the checked worker. The standalone
intent projection borrows worker policy mutably, but the original signed runtime
chain already holds that exclusive borrow. Rechecking a second policy object
would produce another verification, not a binding to the original runtime.
The [composition](../testing/performance/0.74/diagnostic-authority-intent-local-design.md)
therefore borrows the original authority guard and captures only private,
immutable intent pins. Those pins cannot mint a worker or start permit.

Validation brackets the signed expected intent with two original authority
observations. Each authority observation keeps its existing journal, runtime,
journal order. The binding retains the exact signed envelope, full asserted
Reserved state, controller configuration and peer. A changed request, nonce,
controller attempt or signature window refuses even when the replacement has a
valid signature. The same original worker and journal remain borrowed throughout.
Fixed and fixture types cannot convert into one another, clone, cross threads or
expose a start method.

Unbound input and a previously accepted binding have different failure semantics.
A malformed initial controller packet cannot revoke an unrelated healthy worker.
After binding, intent drift or expiry latches the original runtime chain. A safe
journal with the exact original authority pins receives an append-only Policy
refusal. Unsafe or advanced storage is not repaired or adopted. If publication
is partial or its acknowledgment is uncertain, the error keeps both the initial
projection failure and publication uncertainty. Later calls refuse before
additional I/O, even after the caller restores the old bytes or permissions.

The [local tests](../testing/performance/0.74/local-runs/diagnostic-authority-intent-20261011/README.md)
exercise owned Linux helper processes, signature and state changes, authority
advancement, storage drift, independent concurrent chains and four publication
fault checkpoints. Compile-fail examples cover exclusive borrowing, cloning,
fixture promotion and a missing start route. The fixed-dispatch positive case is
a private nonroot model; the fault checkpoints are not physical power-loss proof.

This establishes an original-authority and expected-intent relationship, not the
fixed publisher initially targeted by the package. The original host fence and
trusted persisted Reserved state must still be joined before fixed storage can
publish a consumed intent. Peer, clock and state remain assertions until their
trusted owners are connected. Output creation, original start, watchdog, owned
cleanup and real issuer/host enrollment follow. No cache product, server operation
or frozen qualification input changes, and no throughput or allocation gain is
inferred from this control-plane work.

The thirteen new unit cases pass three retained repetitions. The selected Linux
regression scope passes 465 with one existing ignored; Windows passes 196.
Root checks pass 110 performance contracts, 13 evidence, six gated-target and 23
governance cases; twelve compile-fail doctests pass. Strict lint, MSRV 1.88,
formatting, rustdoc and local documentation/contract checks also pass. The packet
retains 29 raw captures, including absent-API and invocation failures plus the
unchanged qualification and ship refusals. These overlapping local scopes do not
measure product throughput, unique release-wide coverage or fixed-host readiness.

### Retaining the original Reserved state file

The signed intent binds an expected Reserved snapshot, but an expected value is
not proof of its persisted owner. The legacy diagnostic state and host lock live
in worker-owned campaign storage. Promoting that state to root authority or adding
a separate independent lock would not establish the required shared transaction
boundary. A trusted publisher and a common supervisor-owned fence remain
necessary before intent consumption or execution.

The [new read-only guard](../testing/performance/0.74/diagnostic-reserved-state-local-design.md)
addresses one concrete prerequisite: an existing state file under the original
fixed root-owned intent storage. It borrows that storage exclusively, opens only
the fixed name relative to the retained directory and validates the existing
canonical schema-1 envelope. It compares the entire valid Reserved state, not
only its lease, stage or cell. The file must be private, single-link and on the
same device as the original leaf. Reads have a fixed byte limit.

The guard retains the original descriptor, bytes and metadata, including
timestamps and link count. Rechecks compare both the named object and the
original file. A new inode containing identical bytes is still a replacement.
Changing a revision or rewriting content is an owner transition, not an allowed
refresh. Original ancestry checks bracket the file observations; unrelated
directory growth remains harmless. Same-leaf pending state, pending IPC, active
campaign or an IPC ledger not validated by this slice refuses conservatively.
That is not global campaign or IPC admission.

Invalid initial expected state fails before storage I/O. Once inspection begins,
failure latches the original storage, including after the wrapper is dropped or
the caller restores bytes and permissions. No state getter, path/owner/FD selector,
clone, fixture conversion, mutation, lock or start method is exposed. Existing
campaign state, lock behavior, installer and state serialization remain unchanged.

The [local proof](../testing/performance/0.74/local-runs/diagnostic-reserved-state-20261011/README.md)
uses private nonroot modeled ancestry, canonical/hash/schema failures, file
substitution, symlinks, FIFOs, hardlinks, metadata/size drift, exact-state
mutations, pending context and independent concurrent guards. Private seams
exercise read-budget and descriptor errors without adding public selectors.
These observations do not authenticate an initial host, prove continuous
confinement or test a physical durability crash.

The reader neither creates nor migrates Reserved state. Its future trusted
publisher must populate that namespace through the supervisor's common fence;
copying a worker-owned fixture is not enrollment. The original authority,
signed intent, persisted state and fence join remains followed by fixed intent
publication and output/start/watchdog/owned cleanup. No product measurement,
server operation, qualification input or numerical performance claim accompanies
this prerequisite.

Ten final unit cases pass three separately retained repetitions. The final
selected Linux scope passes 476 with one existing ignored, Windows passes 196,
and root checks pass 111 performance contracts, 13 evidence, six gated-target
and 23 governance cases. Fifteen compile-fail doctests, strict lint, current/MSRV
checks and documentation checks pass. The packet retains 31 closed captures.
An expanded private test initially shadowed a helper name; all-target compilation
caught it, and only the test variable was renamed. Early runs remain intermediate
proof, not substituted for the final repeated source. These overlapping scopes
are not release qualification or numerical performance evidence.

## The existing campaign lock must also guard original state

A Reserved-state snapshot alone cannot serialize diagnostic work against a
campaign. An independent lock would protect only its own participants. The next
local control-plane hypothesis was narrower: can the original state reader
retain the very same lock used by both legacy campaign admission paths, reject
object substitution and release cleanly without altering state?

The new guard exclusively borrows the original Reserved reader. From its
retained performance-parent descriptor it opens the fixed campaigns directory
and existing `.host-execution.lock`; it never creates another lock. The directory
must have private trusted ownership, and the lock must be an empty private
single-link regular file on the directory's device. Both original and named
identities remain pinned. State observations are bracketed by lock checks and
conservative refusal of legacy campaign, diagnostic, pending IPC and lifecycle
contexts. Busy on initial acquisition is ordinary contention, not revocation;
unsafe or bound drift permanently refuses the original storage.

The [local evidence](../testing/performance/0.74/local-runs/diagnostic-original-fence-20261011/README.md)
checks acquisition in both orders against the real legacy flock entry points,
concurrent independent opens, replaced descriptors and names, unsafe locks,
directory/state drift, dangling conflict links, sticky refusal and healthy drop.
Ten unit cases pass three separately captured repetitions. No fixed-host
enrollment or performance improvement follows from these functional models.

The negative test matters as much as the lock-contention success. After the
short guard drops, legacy campaign admission can acquire a claim while the
fixed Reserved file still exists, because that route does not read the new
state location. The binding proves short transaction interoperation, not
durable mutual exclusion. The deployed worker-owned campaigns layout is
deliberately refused; the implementation does not chmod it, migrate it or copy
state from it.

Before a trusted reservation publisher can proceed, campaign admission must
consult the same fixed state under the same lock, and a quiescent enrollment
must establish ownership for every participant. Holding an external lock during
the workload would stall supervisor maintenance and is not a solution. Original
authority/intent/state composition, fixed consumption publication and the
output/start/watchdog/owned-cleanup lifecycle remain subsequent packages.

## Campaign admission now checks retained diagnostic context

The previous short-fence test exposed a concrete gap: the campaign path did not
consult the fixed state location after the diagnostic guard released its flock.
The next control-plane hypothesis was that a conservative read-side check in
the real admission/recovery algorithms could close that gap without a publisher,
another lock or an ownership migration.

For the canonical Linux production campaign root, those algorithms now open
existing fixed intent storage after acquiring the common flock. They retain
the original storage ancestry, campaigns directory and named/original lock
metadata across marker handling. Any fixed-leaf entry blocks admission,
including an unknown future name, terminal record, pending publication, FIFO
or dangling symlink. Absence of active-diagnostic.json alone is insufficient.
Missing or unsafe storage refuses; no directory or permission repair follows.

Each observation opens a fresh directory descriptor relative to the retained
leaf before enumeration. Reusing a duplicated descriptor would share its
directory offset and could make a later scan falsely appear empty. Pre/post
checks also catch object or context changes around marker creation/recovery.
A late failure preserves the marker and evidence rather than deleting a partial
transaction or starting a replacement. The short legacy diagnostic lock entry
checks fixed context too; its existing legacy-state transactions remain allowed
when fixed storage is empty.

The [local packet](../testing/performance/0.74/local-runs/diagnostic-fixed-admission-20261011/README.md)
uses the real claim/recovery algorithms with private nonroot factories. It tests
valid canonical Reserved state after fence release, every entry type, repeat
scans, original-object drift, late seeded conflicts, marker retention, normal
legacy recovery and canonical path handling. The first test wrongly expected a
symlink root to be accepted; the existing rejection was preserved and the test
was corrected. No acceptance threshold was changed. The final focused suite
passes 13 cases in each of three repeats with seed `0x747`; final Linux
regression passes 526 cases with one pre-existing real-systemd-bus test ignored.
Windows package tests pass 196 cases and native root contract/registry/governance
tests pass 155. These are functional counts, not throughput measurements or
physical host evidence. MSRV 1.88.0, current 1.94.0, warnings-denied Clippy,
format, rustdoc and compile-fail doctests are retained in the packet.

This closes the production read-side bypass, not the whole diagnostic protocol.
Empty safe storage is not root authority, and the deployed worker-owned campaign
layout remains unenrolled for diagnostic publication. Older supervisor binaries
do not know this predicate. A future publisher therefore requires coordinated
reviewed deployment and trusted ownership enrollment, then original
authority/intent/state composition and publication under the same fence.
Output/start/watchdog/owned cleanup and controlled-host proof remain separate.
There is no server run, numerical performance result or release admission here.

## Original intent and Reserved state share an observation guard

The next control-plane hypothesis was that separately valid authority/intent
and Reserved-state/fence readers could be joined without recreating either
chain. Checking equal lease IDs would be insufficient: the complete signed
Reserved snapshot must match the state reader's original expected snapshot.
The [composition](../testing/performance/0.74/diagnostic-fenced-intent-local-design.md)
therefore borrows both original readers exclusively, keeping fixed and fixture
origins distinct. A wrong unbound join assertion refuses before I/O and leaves
healthy inputs usable.

Revalidation observes fence/intent/fence/intent/fence. Each intent observation
still uses the original authority, signed runtime and journal brackets; each
fence observation retains its original state file, ancestry and common lock.
A bound failure latches both chains. When the fence fails, the authority journal
is checked independently before Policy refusal publication. Unsafe or advanced
authority is not repaired or appended. Failed acknowledgement preserves the
first fence error and typed uncertainty; later calls do no I/O. Intent failures
reuse their existing refusal bridge rather than publish a duplicate record.

A successful join drop leaves the caller's original fence held. Tests cover
that property, full-state mismatch, original name/inode drift, late context
changes, signed-intent drift, partial refusal publication and independent
concurrent chains. The durability test initially tried to reopen a journal while
its original guard still held the lock. Reopen was moved after the original
guards dropped; production exclusion was kept intact.

Two absent-API attempts hit a Rust 1.94.0 resolver internal compiler error.
The [evidence](../testing/performance/0.74/local-runs/diagnostic-fenced-intent-20261011/README.md)
retains those separately from the actual missing-API diagnostic obtained with
MSRV 1.88.0. Compiler failure is not treated as a semantic or performance result.
Final focused tests pass 14 cases in each of three repeats. Linux regression
passes 540 cases with one existing systemd-bus test ignored, Windows passes 196,
native root contract/registry/governance tests pass 156 and Linux compile-fail
doctests pass 22. Current/MSRV and warnings-denied Clippy checks pass. The counts
describe local functional coverage, not a product speedup or installed-host proof.
This joins observations, not authenticated initial host admission or execution.
Peer/clock authentication, trusted publication and output/start/watchdog/owned
cleanup remain before a pilot. No server operation or numerical claim follows.

## Initial reservation can inspect empty storage under the common fence

The existing Reserved reader needs a state file before it can borrow the common
fence. Reusing it to publish the first Reserved record would therefore create a
circular dependency. The next hypothesis was narrower than publication: the
existing trusted Fence engine could protect an original empty-storage observation
without a state file, another lock or ownership migration.

The [new guard](../testing/performance/0.74/diagnostic-empty-reservation-fence-local-design.md)
exclusively borrows the original fixed storage and acquires only the existing
trusted campaign lock. Two fresh scans require the fixed leaf to be empty;
known, pending, terminal, unknown, non-UTF-8 and nonregular entries all refuse.
Original ancestry and named/original fence checks bracket those scans, and
legacy campaign/diagnostic/IPC plus parent lifecycle contexts also refuse.
No file contents are adopted, decoded or repaired.

Initial Busy is ordinary contention and leaves healthy storage and the other
owner intact. Once bound inspection starts, an object or context failure latches
the original storage. Restoring files or dropping the wrapper cannot refresh it.
Healthy drop releases only the guard's own original flock; the empty observation
then expires and creates no durable reservation.

The [local tests](../testing/performance/0.74/local-runs/diagnostic-empty-reservation-fence-20261012/README.md)
exercise both lock acquisition orders against legacy campaign APIs, every entry
kind, unsafe/missing locks, named/original descriptor drift, repeated scans,
seeded between-scan context and independent concurrency. Positive fixed ancestry
is a private nonroot model, not physical host enrollment. This removes the
state-before-lock dependency; signed authority/empty-fence composition and a
no-overwrite publication transition with partial/uncertain/crash behavior remain
next. Peer/clock authentication, trusted ownership and original lifecycle proof
still precede a worker pilot. No numerical or server claim follows.

Final local checks pass 11 focused cases in each of three repeats, 552 Linux
regression cases with one existing systemd-bus test ignored, 196 Windows cases,
157 native root contract/registry/governance cases and 26 compile-fail doctests.
Current/MSRV, warnings-denied Clippy and format pass. A root source guard initially
expected a one-line shared-fence call that rustfmt split; its structural markers
were corrected without changing policy or thresholds, and the red log remains.
These counts establish local functional coverage, not throughput or host admission.

## Signed intent and empty storage can share the original fence

An initial reservation needs two prerequisites at once: signed authority must
still be valid, and fixed storage must still be empty under the common host
lock. The existing state-present join cannot supply that proof before a state
exists. The next local hypothesis was that the original intent and empty-fence
guards could be composed without reopening either origin or releasing the lock.

The [read-only join](../testing/performance/0.74/diagnostic-empty-fenced-intent-local-design.md)
borrows both original guards and checks fence, intent, fence, intent, fence.
Each observation retains its input's original-object and authority brackets.
The full asserted Reserved snapshot must match the signed-intent pins before
composition I/O. Empty storage has no persisted snapshot, so this match does
not establish a durable reservation.

Bound failure latches both original chains. Fence errors attempt Policy refusal
only through independently exact-safe original authority storage. Unsafe or
advanced authority is not appended; the first fence error and acknowledgement
uncertainty remain typed. Intent errors use their existing refusal bridge once,
not a duplicate append. Already refused inputs cross-latch before I/O. Dropping
a healthy join leaves the caller's original empty fence held.

The [local evidence](../testing/performance/0.74/local-runs/diagnostic-empty-fenced-intent-20261012/README.md)
covers expiry, a canonical envelope with an invalid signature, state/peer/worker
drift, new leaf entries, late context, partial Policy publication and independent
concurrency. All 14 focused cases pass in each of three repeats. Linux regression
passes 567 cases with one existing real systemd-bus test ignored; Windows passes
196 and Linux compile-fail doctests pass 30.
Native root contract/registry/governance tests pass 158; current/MSRV,
warnings-denied Clippy, format, documentation checks and mdBook also pass.
These checks preserve the existing qualification and ship refusals.
The functional observations use synthetic keys and private nonroot modeled
storage, not physical enrollment,
continuous confinement or performance measurements.

The initial implementation had a nine-versus-ten lifetime argument error.
Rust 1.94 also failed while rendering that diagnostic. The corrected source
passes the focused suite; the initial compiler log is retained separately.
The actual absent-public-API red was obtained with MSRV 1.88 before implementation,
rather than inferred from an internal compiler failure.

This confirms the read-only composition hypothesis, not safe initial publication.
The next step must transition this same original empty fence into no-overwrite
publication while keeping the lock and retaining the exact published object.
A write followed by empty-guard revalidation necessarily refuses; drop/reacquire
would leave a race. Partial/crash uncertainty, authenticated peer/time and
coordinated trusted ownership/binary rollout still precede a worker pilot.
No product speedup, server admission or release qualification follows.

## Initial Reserved publication keeps the original fence and created inode

The read-only empty-fence join solved observation, not persistence. The next
hypothesis was that an initial Reserved record could be published under that
same original lock without overwriting a competing or orphaned record.
The [fixed-only publisher](../testing/performance/0.74/diagnostic-reservation-publication-local-design.md)
consumes the original empty fence and retains its lock while exclusively
borrowing original fixed authority and signed intent. Only a matching initial
Reserved snapshot with revision one and completed cell zero is eligible.

The initial record uses descriptor-relative O_CREAT and O_EXCL, not the
existing replacement writer. An existing record is never adopted, even when
its canonical bytes match. Creation may be visible before writing finishes:
the campaign admission already rejects every fixed-leaf entry, so a partial
record is a conservative exclusion barrier rather than evidence of success.
This requires coordinated deployment of that admission check, not a new lock
or an automatic ownership repair.

The publisher retains the created descriptor and existing canonical schema-one
envelope. Private mode, owner, group, device and one link are checked before
writing. File and directory sync precede acknowledgement, which also requires
exact original/named inode, bytes, fixed-leaf inventory, ancestry, common fence,
authority and signed-intent observations. The successful runtime guard retains
the created document and original lock; no path or stream FD is exported.

A failure keeps any created state and owns the original lock until drop.
Bound failures block both original chains. Policy refusal reaches only the
independently exact-safe original authority journal, preserving first error
and uncertain acknowledgement. Intent refusal is not appended twice.
Restoring an inode, bytes or permissions cannot revive the same guard.
Neither successful nor failed drop removes state or cleans up an uncertain
attempt. A fresh empty-fence guard refuses retained context.

The [local checks](../testing/performance/0.74/local-runs/diagnostic-reservation-publication-20261012/README.md)
exercise eight logical interruption boundaries, including partial bytes,
file sync, directory sync and the final acknowledgement boundary. They also
cover late same-name creation, inode/permissions/link drift, signature/state/
peer/worker refusal, unsafe or advanced authority, partial Policy publication
and concurrent independent chains. These are synthetic nonroot fixture
observations, not machine power-loss or installed-host durability proof.
Initial test compilation errors are retained separately from the passing suite.

This implements the local initial-state publisher, not backend activation.
The service has no publication entry and peer/time remain external assertions.
A surviving valid state can be inspected read-only but cannot recreate a lost
runtime acknowledgement. Exact intent-consumption publication and restart
ambiguity need a separate contract, followed by authenticated peer/clock,
trusted ownership/binary rollout and original output/start/watchdog/owned
cleanup. No performance result or release qualification follows.

Linux warnings-denied Clippy initially rejected the large error variant because
it owned the entire fence/storage container, and found a missing test assertion
using the journal variable. The error now boxes that private owned container only
when constructing a failure; the success path retains its original storage value.
The change neither reopens the fence nor changes the write/sync protocol.
The red lint log is retained, and the final source is rechecked separately.

The final source passes all 15 cases in each of three repeats. Scoped Linux
regression passes 583 cases with one existing real systemd-bus test ignored;
Windows passes 196, native root contracts/registry/governance pass 159 and Linux
compile-fail doctests pass 34. Current/MSRV, warnings-denied Clippy and format
pass. These counts establish local functional coverage, not throughput,
allocation improvements or physical-host readiness.

## Consuming one intent does not grant permission to execute

Initial Reserved publication leaves a runtime acknowledgement, not merely a
readable file. The next hypothesis was that cell-one intent consumption can
retain that original acknowledgement and common lock across the entire journal
write, without reopening storage or making a recovered record eligible to start.

The fixed-only library entry consumes the original publisher by value. It checks
the full initial state and signed intent before mutation, then exclusively creates
the cell-one JSON record and digest commit under the original directory descriptor.
It reuses the existing retained-intent schema, canonical newline encoding and size
bounds. Only those new owned files are sealed from private 0600 to read-only 0400.
The original Reserved state is unchanged; consumption does not advance a cell.

A private namespace transition permits only the original state and the exact
objects actually created by this attempt. It does not accept a caller-selected
allowlist, existing history or a same-name replacement. Checks interleave original
authority/intent with ancestry, common fence, legacy context, exact inode, owner,
group, mode, links and bounded bytes. File and directory sync precede the final
acknowledgement bracket. Both success and failure keep the original lock; failure
retains every created byte and cross-latches bound originals. Policy refusal uses
only independently exact-safe original authority and preserves uncertainty.

The [local contract](../testing/performance/0.74/diagnostic-initial-consumption-local-design.md)
separates this functional hypothesis from performance attribution. Baseline is
`175d2dae1509d1b664efe0852074665cc99b82b7`; the public API first fails an absent-entry
test. Fourteen logical checkpoints cover creation, partial bytes, sealing, file
sync, commit creation, directory sync and acknowledgement. Additional cases cover
late targets, unknown and legacy context, substitution, hard links, intent expiry,
signature/peer/worker drift, unsafe or advanced authority, sticky refusal, same-guard replay
and independent concurrent chains.

An expanded test initially expected the same error class for legacy campaign
context as for retained intent-leaf context. Both reject admission, but they belong
to different original-fence checks. The test now preserves the stricter semantic
requirement that neither permits entry, without inventing a common error variant.
Warnings-denied Clippy also rejected a test-only manual range pattern; it was
corrected without a lint waiver. Both negative captures remain in the
[local evidence](../testing/performance/0.74/local-runs/diagnostic-initial-consumption-20261012/README.md).

This validates only the initial consumption protocol locally. Checkpoint errors
are not actual SIGKILL, machine power-loss or installed-host evidence. A surviving
valid pair cannot recreate the lost runtime acknowledgement after restart.
Actual owned-process interruption testing is next; cells two through four and
state advancement need their own contract. Authenticated peer/time, trusted
ownership and binary rollout, original output/start/watchdog/owned cleanup and a
reviewed host rehearsal still precede activation. No server, product workload,
qualification, shipping or numerical performance claim is enabled.

All 12 new cases pass in each of three identical-source repeats. Scoped Linux
regression passes 596 with one existing real systemd-bus case ignored; native
Windows passes 196 portable cases, root contracts/registry/governance pass 160,
and Linux compile-fail doctests pass 38. Current/MSRV, warnings-denied lint,
format, Rustdoc and documentation checks pass. These are local functional
observations, not throughput, allocation or production-readiness claims.

## Actual SIGKILL at diagnostic journal boundaries

The next check tests process loss, rather than returning an injected error from
a writer callback. The [intermediate server plan](../testing/performance/0.74/diagnostic-server-readiness-intermediate-plan-20261012.md)
keeps this local P0 proof separate from later fixed lifecycle, trusted peer/time,
production execution and installed-host rehearsal.

A nonroot parent owns each test subprocess and all temporary account, authority
and fixed-storage roots. The child composes the original signed runtime chain,
holds the existing common fence and stops at a named publication boundary.
Only after the readiness signal and original lock contention are observed does
the parent kill that exact child through its owned process handle. It verifies
signal 9, helper termination through a retained pidfd and original lock release.
This is a kernel SIGKILL, not a returned storage error or a broad process-name kill.

The matrix covers eight initial-state stages and fourteen consumption stages,
including empty/partial objects, file sync, sealing, directory sync and the final
acknowledgement boundary. Death after each successful API return tests loss of
the runtime acknowledgement. Two uninterrupted controls distinguish normal
drop from signal death. Every case performs two fresh read-only process
inspections and compares exact retained bytes, inode, owner/group/mode/link count,
length and mtime/ctime. Actual atime is deliberately not an identity field.
Fresh inspection never reconstructs a successful runtime publisher or consumer.

The first process run failed because libtest prefixes readiness output with the
test name. The strict parser originally expected the marker at the line start
and timed out; the timeout is a harness failure, not durability evidence.
The correction accepts the exact libtest framing, rejects malformed/ambiguous
messages and has separate fast regression cases. A warnings-denied Clippy
failure then simplified that parser's boolean without suppression. Negative
captures remain in the [P0 packet](../testing/performance/0.74/local-runs/diagnostic-process-crash-20261012/README.md).

The before-create case has no durable mutation and leaves empty-fence eligibility.
It does not prove global request/nonce replay protection before the first record.
That earlier boundary must be specified with P1 lifecycle. For every retained
state, fresh empty reservation refuses even when the record/digest pair is
complete. Surviving valid bytes are not the lost runtime acknowledgement.
This proof does not cover machine power loss, reboot, production issuer keys,
trusted root enrollment, original worker start or complete cgroup cleanup.
The product candidate and qualification/ship boundaries remain closed.

The final source passes all 24 SIGKILL cases and two uninterrupted controls in
each of three repeats, with 52 fresh-process observations per repeat. These are
local process-loss observations, not throughput, allocation, power-loss or
production-readiness results. The original writer implementation and durable
formats are unchanged; new path selectors and process helpers exist only in
the crate's test build. The manual gates remain separate from ordinary fast CI.

## Checking adjacent diagnostic state transitions

After the process-loss proof, P1 starts by distinguishing a valid snapshot from
a valid transition. Two individually valid states can still skip a cell, change
the original binary/boot, forget an inode or overwrite a stop reason. The
[transition contract](../testing/performance/0.74/diagnostic-transition-local-design.md)
therefore validates the previous and next state together. The local model
publisher invokes it under its existing host fence before creating pending state.

Revision advances by exactly one. Lease, boot, binary, build provenance and
reservation origin stay fixed; controller refresh cannot revive an expired
controller. A running cell retains its original start and any bound inode.
Only Running may complete one cell, returning to Reserved for the next of four
cells or reaching Terminal after the fourth. Stop reason is sticky, and Terminal
has no subsequent publication. Invalid and stale requests leave journal bytes
unchanged. Existing schema-one encoding, snapshot readers and recovery formats
are unchanged.

The test hypothesis is functional: enforce the graph before mutation without
breaking existing model lifecycle, cancellation and concurrent host admission.
The [local evidence](../testing/performance/0.74/local-runs/diagnostic-transition-20261013/README.md)
retains the missing-function red test, all 25 stage pairs, deterministic seed
`0x074e`, 64 four-cell histories per repeat and invalid revision/cell mutations.
Separate tests exercise identity/time/start/inode/reason drift, expiry, overflow,
terminal replay and publication refusal. An initial graph fixture accidentally
gave a pre-start cancellation a process identity; it was corrected while
preserving the stricter rule, and its negative log remains available.

This does not make the fixed runtime guards advance. They deliberately retain
the initial state and signed-intent binding, so reopening or rebinding changed
files would undermine the original-object guarantee. Fixed cells two through
four need an owned progression protocol, retained signed history and early
durable attempt/nonce admission. A caller-asserted successful outcome is still
not writer cessation or empty-cgroup proof. P1 remains open, as do trusted
production inputs, execution/watchdog/cleanup and installed-host rehearsal.
No performance gain or release/qualification permission follows from this slice.

The frozen implementation passes all eight new cases in each of three Linux
repeats. Scoped Linux regression passes 607 with four manual cases ignored;
Windows passes 204, root contracts/registry/governance 162, and Linux compile-fail
doctests 38. Current/MSRV checks, warnings-denied lint, format and documentation
checks pass. No expensive workload or server operation was performed. The
inherited qualification digest mismatch and closed ship gate still refuse;
their inputs and thresholds were not changed to make this package pass.

## Retaining the diagnostic attempt before state publication

The next P1 hypothesis concerns admission, not throughput. A process can die
after signature checks but before publishing Reserved. If no byte was retained,
there is no durable evidence of an accepted request. Moving another marker
earlier cannot prove a nonce consumed before its own first write. The
[early-attempt contract](../testing/performance/0.74/diagnostic-early-attempt-local-design.md)
therefore makes the acceptance boundary explicit: no acknowledgement before
exclusive creation, complete signed-attempt bytes, owned-file sealing, sync and
final original authority checks.

The guard borrows the original fixed intent and owns its original common fence.
It creates only `diagnostic-attempt-074.json`, using the existing retained-intent
schema and byte budget. It tracks actual partial bytes and the original file,
rejects foreign objects and unknown context, and seals only its owned descriptor
0400. Failure retains the lock until drop and publishes refusal only through an
independently safe original authority journal. A changed or advanced journal
cannot receive a new refusal just because the attempt failed.

The [local proof](../testing/performance/0.74/local-runs/diagnostic-early-attempt-20261013/README.md)
checks both returned failures and actual SIGKILL. BeforeCreate is empty and
unadmitted. At later boundaries an empty, partial or complete record remains
admission-blocking. AfterReturn death loses the runtime acknowledgement despite
valid sealed bytes. Fresh read-only processes inspect retained context; none
reconstruct a writer, repair history or execute a cell. Exact lock release and
owned helper termination are checked separately from retained bytes and metadata.

Early test mistakes are retained: an unsupported Clone assumption in the peer
fixture, unused variables rejected by warnings-denied lint, and a root-contract
literal that depended on formatter line breaks. Corrections changed test setup
and checks, not the signed protocol, thresholds or refusal requirements.

This primitive does not yet advance to Reserved or consume cells two through
four. That transfer needs an owned protocol retaining the original attempt,
authority and fence, rather than reopening files or rebinding changed state.
It is local attempt retention, not global nonce protection, authenticated clock/
peer, machine power-loss, production enrollment or performance improvement.
The intermediate plan and product qualification/ship boundaries remain closed.

At the frozen source, all twelve functional cases pass in three repeats.
Three process repeats each pass nine SIGKILL cases and one uninterrupted control,
with twenty fresh-process inspections. Full process regression also reruns the
original twenty-four P0 signal cases and two controls through the extended
harness. Linux scoped regression passes 619 with five manual cases ignored;
Windows passes 204, root contracts 163 and Linux compile-fail doctests 41.
Current/MSRV, warnings-denied lint/Rustdoc, format and documentation checks pass.
These results establish local refusal/retention behavior, not throughput,
allocation savings, host power-loss durability or production readiness.

## Moving the original attempt into state publication

The next P1 step connects the accepted attempt to Reserved without treating
surviving bytes as a new acknowledgement. The
[transfer contract](../testing/performance/0.74/diagnostic-attempt-publication-local-design.md)
requires a consuming API: the caller loses the original early-attempt guard,
and the publisher receives its existing fence and original sealed descriptor,
stamp and expected bytes. No lock is released/reacquired and no file is opened
to reconstruct trusted state. Opens during revalidation compare names with the
already owned descriptor rather than adopting a replacement.

This distinction also governs the namespace. A general allowlist of
`diagnostic-attempt-074.json` would let an unrelated same-name file survive
admission. Instead the name is permitted only while its original document is
held and revalidated. Publication first requires exactly the owned attempt,
then attempt plus its exclusively created initial state. The cell-one consumer
inherits the same document and checks it throughout record/commit publication.
Attempt bytes, inode, ownership, mode, link count and mtime/ctime cannot drift;
unknown objects or legacy campaign context still refuse.

The [local evidence](../testing/performance/0.74/local-runs/diagnostic-attempt-publication-20261013/README.md)
tests normal ownership transfer, unbound mismatch without revocation, partial
state retention, late targets, original-object/intent drift and refusal
uncertainty. It also confirms that unacknowledged internal storage cannot mint a
publisher and that the legacy publisher does not adopt an attempt filename.
Actual SIGKILL phases cover both state publication and cell-one continuation,
including death after each return. Before state creation, the original accepted
attempt already blocks fresh reservation; all fresh inspection remains read-only.

Clippy rejected the original internal failure representation because it returned
256 bytes by value. A Box now retains that exact original storage only on the
failure path, with no altered ownership rule or lint suppression. An older
source-level guard also required the explicit legacy `only_state` call; the
call remains explicit in that branch rather than weakening the legacy check.
These structural corrections are retained as negative observations, not
allocation or throughput wins.

No durable format or filename changes, and the state remains initial Reserved.
This completes only the early-attempt-to-initial-state/cell-one connection.
Fixed cells two through four, signed state/history progression and incomplete/
reboot semantics remain open. Caller outcomes are not proof of writer cessation
or an empty cgroup; production inputs, execution/watchdog/cleanup and dedicated
host rehearsal still precede activation. Product qualification and shipping
remain separate closed boundaries.

At the frozen source, all fourteen functional cases pass in three repeats.
Each process repeat passes twenty-four SIGKILL cases and two controls, with
fifty-two fresh read-only inspections. The full prior-phase regression passes
six manual tests, covering fifty-seven signal cases and five controls. Scoped
Linux regression passes 633 with seven manual/host cases ignored; Windows
passes 204, root contracts 164, and Linux compile-fail doctests 43. Current/MSRV,
warnings-denied lint/Rustdoc, format and docs checks pass. These are local
ownership/retention/refusal observations, not throughput, allocation, host
durability or production-readiness claims.

## Checking signed history without authorizing the next cell

P1 needs to separate a consistent history from permission to continue. The
[history contract](../testing/performance/0.74/diagnostic-signed-history-local-design.md)
introduces bounded byte inspection of up to four retained record/commit pairs,
anchored to the exact initial attempt. It verifies every signed Reserved state
and unit projection under the original controller, peer and worker policy.
Cell indices, lease/boot/binary/provenance, reservation origin, previous raw
hashes, commit bytes and unique request/nonce must agree. Malformed,
noncanonical, oversized, incomplete or conflicting input refuses and latches
the supplied original intent guard.

An increasing revision alone is insufficient. Returning from one Reserved cell
to the next requires at least Reserved to Starting to Running to Reserved:
three publications, not one. The checker therefore requires a minimum gap of
three revisions while allowing heartbeat publications. Observation and
controller clocks cannot regress, and the supplied current state must be
consistent with the retained prefix. Re-signed mutations test these rules even
when all affected outer hashes and commit markers are recomputed.

This remains a necessary bound, not proof that omitted transitions occurred.
The previous-record hash is an outer record field, not part of the signed intent
body. Its integrity link cannot replace an authenticated transition trace,
actual cell outcome or demonstrated writer cessation. A valid signature over a
later Reserved projection must not let a caller skip the owned completion
protocol. The new result deliberately contains only a pair count and hashes;
it has no append, execution or recovered-acknowledgement conversion.

Historical signatures are checked at their signed issue time, with no future
issue time relative to the asserted current clock. The current original intent
is independently revalidated before and after inspection. A new valid current
intent can inspect expired historical records, but history cannot renew an
expired current intent. Independent concurrent inspections share immutable
bytes, not a runtime acknowledgement. Peer/time remain assertions until P2.

Existing fixed publishers and filesystem fences are unchanged. No write,
process spawn, new filename, durable schema or server route is added, so this
slice adds no process-loss boundary. It does not establish original-object
ownership, successful completion, durable refusal across restart or host
rollback protection. Fixed cells two through four, incomplete/reboot handling,
trusted inputs, execution/watchdog/cleanup and host rehearsal remain open.
These checks improve tooling correctness; they are not product throughput or
allocation improvements and do not change qualification or ship admission.

The [exact-source packet](../testing/performance/0.74/local-runs/diagnostic-signed-history-20261013/README.md)
retains the missing-entry compilation failure and separates the initial
nine-test run from the final eleven cases. Each of three final-source repeats
passes all eleven tests, including 64 seeded legal chains and 64 re-signed
mutations. Scoped Linux regression passes 644 with seven manual/host cases
ignored; Windows passes 204, root contracts 165 and Linux doctests 44.
Formatting, current/MSRV checks, warnings-denied lint/Rustdoc and docs/book
checks pass. No process-loss or server result is claimed for this read-only API.

## Retaining an owned departure without replacing the initial state

The next P1 slice moves from byte inspection to one owned durable transition.
The [Starting intent contract](../testing/performance/0.74/diagnostic-starting-intent-local-design.md)
consumes the acknowledged initial cell-one guard, which already holds the
original attempt, initial Reserved state, record/commit and common fence.
The candidate must be the exact adjacent revision-two Starting departure;
skipped revisions, completed cells, changed identity/origin, cgroup inode,
stop reason or invalid controller/clock bounds refuse before creation.
This validation still operates on asserted time, not production clock authority.

The initial active state is deliberately not overwritten. A separate
`diagnostic-starting-intent-074-v1` record links its full encoded bytes,
cell-one record and original signed envelope to the complete next state.
Those hash links are not additional controller signatures. The record
establishes an owned departure intent, not a live executing state or a completed
cell. Old readers and fresh reservation reject the retained context rather
than infer a new permission from it.

Exclusive creation retains the actual descriptor before namespace checks.
Partial/full writes update the expected owned prefix; sealing to 0400,
file/directory sync and final original-chain checks precede acknowledgement.
The directory scan permits the new name only while that exact document is
owned and revalidated, not through a general allowlist. The old attempt, active
state and cell-one objects remain byte-for-byte and metadata-stable. Their
replacement, mode/link/byte drift, foreign objects and late target appearance
refuse. The original borrow and lock survive in either the result or failure;
there is no unlock, reacquisition, adoption, repair or duplicate append.

Logical interruptions cover all eight departure publication boundaries.
Actual SIGKILL also covers death after successful API return, so acknowledgement
loss cannot be mistaken for an incomplete-only scenario. At BeforeCreate,
the original attempt/state/cell-one context already exists: fresh reservation
must refuse even though the departure file is absent. Every terminated case
checks original lock release, exact owned helper termination and two fresh
read-only processes with unchanged retained bytes and metadata. The prior
process matrices remain separate regression coverage.

Development negatives include missing-entry compilation, generic I/O errors
requiring conversion through the storage error type, a missing test-only
authority-stage alias and a source guard that depended on formatter line breaks.
The corrected guard ignores whitespace while requiring the same sync call.
No transition, refusal requirement, budget or admission threshold was relaxed.

The opaque result only revalidates this original chain; it cannot return a prior
guard, export a path/descriptor, start a worker or mark completion. Original
intent expiry remains a refusal, not an opportunity to renew consent from the
stored hashes. Later lifecycle must distinguish historical origin from current
cell authorization and authenticated outcome/cleanup before advancing. Fixed
cells two through four, trusted peer/time/issuer inputs, output/executable/
process/cgroup binding, autonomous watchdog and host rehearsal remain open.
This is local tooling correctness and process-loss evidence, not throughput,
allocation, physical power-loss durability or production-readiness improvement.

The first concurrent repeat batch also retained two readiness-timeout failures
in the owned-process harness; they are not completed crash matrices. Heavy
overlapping functional and process checks are an observed scheduling condition,
not a numerical attribution of the cause. The 60-second readiness timeout and
release thresholds remain unchanged; final process repeats use bounded
single-flight scheduling.

A full Linux batch additionally exposed a transient `Busy` in an older
transition-publisher fixture between reservation and immediate lock
reacquisition. The fixture now acquires its original fence once and calls
`reserve_fenced`, holding that same object through candidate validation.
The invalid/stale-state assertions and underlying product lock are unchanged.
This removes the fixture's release/reacquire window rather than retrying or
weakening the lock rule. Its correction is kept separate from the new runtime
Starting-intent implementation and from final evidence.

The [final-source packet](../testing/performance/0.74/local-runs/diagnostic-starting-intent-20261013/README.md)
records implementation `07008eb4` and the separate test-only fence correction
`099bb14d`. All sixteen new functional cases pass in three final-source
repeats. Each process repeat passes nine SIGKILL cases and one uninterrupted
control, with twenty fresh read-only inspections. Final regression reruns all
seven process proofs: 66 signal cases, six controls and 144 fresh inspections.
Linux scoped functional tests pass 660 with eight manual/host tests ignored;
Windows passes 204, root contracts 166 and Linux compile-fail doctests 48.
Current/MSRV checks, warnings-denied lint/Rustdoc, format and docs/book checks
pass. No inherited negative was removed and no qualification input was changed.

## Declining an incomplete attempt without claiming cleanup

The next lifecycle hypothesis was that an owned Starting attempt can be
permanently declined without renewing its original launch consent or interpreting
a missing PID/cgroup inode as proof that no writer exists. The
[quarantine design](../testing/performance/0.74/diagnostic-starting-quarantine-local-design.md)
uses a consuming runtime transition: the original guard becomes an opaque
quarantine that still owns its borrow, retained objects and common fence.

The operation first latches all original readers, then attempts a Policy refusal
through the original authority journal under its captured trust and host pins.
It accepts no new signed envelope, asserted peer/time, path or completion outcome.
The original attempt, active-state anchor, cell-one pair and Starting document
remain untouched. Even replacement of a Starting document cannot turn revocation
into adoption or authorize repair. The authority record/commit format is unchanged.

Three outcomes answer different questions. Acknowledged establishes a validated
original authority refusal, including an already durable refusal without another
append. Uncertain preserves the exact persistence error, including a durable
commit whose acknowledgement failed. PreviouslyRefused performs no additional
I/O and makes no claim about whether a prior refusal was durable. All three
prevent returning a healthy lifecycle capability. None proves completed execution,
writer cessation or an empty original cgroup, and none authorizes cleanup.

The local tests cover original object conservation and common-lock contention,
the four existing logical authority append faults, unsafe storage and advanced
epoch, replaced attempt objects, expired consent, no retry after prior refusal
and independent concurrent chains. A changed or partial context remains ineligible
for fresh reservation after the runtime quarantine is dropped. Compile-fail
examples prevent cloning, launch, resource release and reuse of the consumed guard.

This is a fail-closed lifecycle improvement, not an allocation or throughput
result. The prior Starting-file SIGKILL matrix does not prove process loss during
authority append. That distinct fault proof and reboot handling remain follow-up
work before later cells, trusted production inputs and execution/watchdog/cleanup
can be admitted. No server workload or qualification threshold changes here.

Implementation `9875fa44` passes all eight cases in three same-source
repeats. Scoped Linux regression passes 668 tests with eight manual/host cases
ignored; Windows passes 204, root contracts 166 and Linux doctests 52. Current
and MSRV checks, warnings-denied lint/Rustdoc, format and documentation checks
pass. The [packet](../testing/performance/0.74/local-runs/diagnostic-starting-quarantine-20261013/README.md)
also retains the first compiler failure. Rust 1.94 stopped in diagnostic rendering
while checking a macro-expanded test; MSRV reported an unused mutable binding.
Removing that unnecessary binding allowed the current compiler to proceed.
This sequence does not establish the compiler bug's root cause, and no warning
policy or correctness assertion was weakened.

## Actual process loss during authority refusal

Logical append errors do not establish what survives an abruptly terminated
owner. The next hypothesis was that the existing owned Starting quarantine
remains fail-closed after actual SIGKILL, including after durable refusal but
before its runtime acknowledgement can be used. The
[process-loss design](../testing/performance/0.74/diagnostic-quarantine-crash-local-design.md)
keeps the original protocol and examines its existing boundaries.

A scoped thread-local observer, compiled only in tests, blocks the exact owned
test process at the four authority append checkpoints. The parent kills only
that child and retains the helper's original pidfd. Additional cases die before
invoking refusal or after successful quarantine return. The original common
fence must be busy before death and released afterward, and the exact owned
helper must become terminal. This is a local nonroot fixture, not a production
worker start or proof of an empty campaign cgroup.

The retained states have different meanings. BeforeRefusal retains the active
revision-one authority, but the five acknowledged attempt/state/cell-one/Starting
files already prohibit a new reservation. An empty record, a sealed record with
no marker, or an empty marker is an invalid journal tail. DurableCommit and
AfterReturn retain the sealed revision-two authority refusal. Even that complete
pair cannot reconstruct the dead owner's runtime acknowledgement or a healthy
Starting guard.

Two fresh processes inspect both reservation eligibility and the journal under
test-parent selected fixture pins and current local host/namespace context.
They do not enroll production trust, append, repair, adopt or retry. Exact
retained bytes and file metadata must stay unchanged after owner termination
and each inspection. A recovered refused view is observation data, not cleanup
permission or proof that every possible writer has stopped.

Development negatives include a temporary string lifetime error in the private
fresh-inspection label and a source guard that stops scanning at the first
cfg(test) declaration. Moving the hook declaration beside the existing test
blocks preserves the same guard markers; the lock, budgets, write/sync order
and assertions are unchanged. Both failures remain in the evidence rather
than being counted as completed crash matrices.

The frozen numerical qualification inputs are unaffected. Actual process loss
does not establish physical power-loss durability, host reboot handling,
production issuer enrollment or authentic cell completion. Those remain
separate boundaries before cells two through four, execution and server admission.

Implementation `86a52311` passes three final-source matrices:
eighteen SIGKILL cases, three controls and forty-two fresh inspections in total.
The full eight-proof regression separately passes 72 signal cases, seven controls
and 158 fresh inspections. A normal all-features library build has no observer
symbols, confirming that this instrumentation is absent for that build/config.
The [exact-source packet](../testing/performance/0.74/local-runs/diagnostic-quarantine-crash-20261013/README.md)
preserves the initial errors and binds the implementation, original storage
protocol and frozen qualification inputs. The local process-loss hypothesis
is confirmed for these boundaries, not for reboot, power loss or production
writer cleanup.

Scoped Linux regression passes 673 tests with nine manual/host cases ignored;
the eight process proofs are run separately, while the real-host bus case is
not run here. Windows passes 204, root contracts 166 and Linux doctests 52.
Current/MSRV checks, warnings-denied lint/Rustdoc and local performance-contract
checks pass. These results confirm local tooling behavior; they add no numerical
allocation or throughput claim and do not open qualification.

## Fresh context claims do not revive an old attempt

The next hypothesis concerns a reader after the original owner has died. A
complete journal can still describe the old authority, but changing a boot,
machine, namespace or trust assertion must not make that authority current.
The [context replay design](../testing/performance/0.74/diagnostic-context-replay-local-design.md)
tests the existing refusal rules without adding another recovery capability.

Eight explicit test-only profiles separate the unchanged case from seven
single-component mutations: boot ID, machine ID, user namespace, mount namespace,
policy digest, epoch and issuer. The original files are never modified to fit
the new claim. Unknown profile names are rejected rather than falling back to
the original profile. Seeded tests check that each mutation changes exactly one
host identity or trust component while preserving a valid input shape.

After each owned SIGKILL boundary, two fresh processes per profile inspect
reservation eligibility and the authority journal. A complete journal with
different claims must return Binding, whether its old authority was still
active or already refused. An incomplete record/marker pair returns Invalid
before binding. That result establishes refusal of a malformed journal, not
proof that the changed boot or issuer was evaluated. The unchanged complete
journal remains readable observation data, never a reconstructed Starting guard.

Every profile still encounters the retained attempt/state/cell-one/Starting
context that denies a new reservation. Original bytes and metadata remain exact
after each read, and the original common fence and helper terminal checks still
apply. No inspection appends a refusal, repairs a tail, advances epoch, retries
the attempt or releases resources based on a missing PID.

Boot and namespace changes are modeled claims inside real fresh processes.
The host is not rebooted and no namespace switch is performed. Fixture-selected
keys and policy pins are not independently enrolled production trust. These
limitations keep the result separate from P2 trusted inputs, P3 writer/cgroup
cleanup and the separately approved exact-host rehearsal. Native/RESP hot paths
and frozen numerical qualification inputs are untouched.

The [exact-source packet](../testing/performance/0.74/local-runs/diagnostic-context-replay-20261013/README.md)
pins implementation `4302202b12d90f0708c029929b6f7f51c6f38aea`.
Three targeted repeats pass 18 SIGKILL cases, three uninterrupted controls and
336 fresh reader inspections. Each repeat separates 56 complete Binding
refusals, 48 partial-tail Invalid refusals and eight unchanged complete views.
This confirms the modeled refusal hypothesis; it does not supply throughput
or allocation measurements, real reboot evidence or permission to continue an
old attempt.

The separate nine-proof regression passes 78 SIGKILL cases, eight controls and
270 fresh inspections, including every prior owned process-loss proof. It is
a regression group, not additional samples merged into the three targeted
repetitions. The next P1 boundary is authenticated completion and later fixed
cells; actual reboot semantics and P2 through P5 remain open.

## Original process exit does not prove cell completion

The completion contract needs more than a plausible terminal state. The owner
must bind its acknowledged Starting record and exact signed intent to the
authenticated original executable, environment, streams, generation and cgroup.
Successful output validation, original exit status, revocation of all writers
and emptiness of the original owned cgroup must precede a completion
acknowledgement. A signature over caller assertions does not establish those
facts, and a fresh reader cannot recover the dead owner's acknowledgement.

The [completion boundary design](../testing/performance/0.74/diagnostic-completion-boundary-local-design.md)
exposes a dependency in the implementation sequence. FixedStartingIntentRead
retains the first intent but has no authenticated start producer.
LiveExecutionRead checks consistency only; TreeSnapshot.empty_at_read is an
observation, not revocation. Authenticated P1 outcome therefore depends on P2
trusted admission and the P3 original-start/writer-revocation producer. Changing
the state validator or appending a Completed record cannot fill that gap.

ProcessRead now has a strict read-only terminal observer. It uses only the
original zero-flag pidfd, checks retained/named procfs root and boot before and
after one nonblocking poll, and revalidates the original generation if no event
is present. Exactly three terminal masks are recognized: POLLIN, POLLHUP and
their union. Unknown bits, inconsistent counts and observation failures refuse.
After exit it does not open a replacement /proc/PID, signal, reap or write.
Its generation is cached original-pin data, not proof of final cgroup membership;
its terminal view contains no successful outcome or release capability.

The [Linux manual](https://man7.org/linux/man-pages/man2/pidfd_open.2.html)
describes exit and hangup readiness for the retained process. It does not extend
that notification to descendants. The local manual fixture holds pidfds for
both an owned Python leader and its descendant, then kills only the leader.
After signal-nine death and a terminal original pidfd, the descendant still
responds to its retained control pipe and writes to stdout. Three same-source
repetitions observe that post-exit write. Only explicit control EOF terminates
the descendant; this is fixture cleanup, not production cgroup-release proof.
The hypothesis that leader exit alone establishes writer cessation is rejected.

The fixture does not trust the child-supplied numeric PID as ownership. Kernel
PPid is checked between both retained generation checks before the descendant
is placed in the signal cleanup set. A separate negative test keeps two owned
but unrelated processes alive and rejects adopting either as the other's child.
The control pipe remains independent of leader wait until explicit closure,
so leader cleanup does not silently stop the writer and mask the counterexample.

The first fast run retained a separate negative result: two tests expected
hangup immediately after owner wait, but WSL kernel 6.6.87.2 continued returning
readable exit. Owner wait and exact readiness are now checked independently;
the accepted masks and error refusals are unchanged. Synthetic masks exercise
hangup classification without pretending that this kernel supplied a hangup.
All 65536 masks are covered, along with first-error ordering, late boot drift,
live-generation refusal, actual graceful/SIGKILL exit and replacement exclusion.

No new persistent write boundary is added. The old Starting publication
regression passes nine SIGKILL cases, one control and twenty fresh inspections.
Later-cell publication, authenticated outcome and new completion-boundary
SIGKILL/lost-acknowledgement matrices remain implementation work after the
original-start producer is connected. Product hot paths and frozen numerical
qualification inputs remain unchanged.

The [exact-source packet](../testing/performance/0.74/local-runs/diagnostic-completion-boundary-20261013/README.md)
pins implementation `fb6afed217ed6193f2898001bbd58da80ecdfc83`. Seven fast tests pass
three repetitions, including foreign-descendant refusal. Scoped process tests
pass 103, diagnostic integrations 65 and Linux doctests 55. These results verify
the observation boundary, not an enrolled producer for P1 completion.

### A sole collector writer and the pipe EOF boundary

The next P2/P3 question was not whether the leader had exited, but who could still
modify its evidence. The old diagnostic unit appends stdout and stderr directly
to regular files in a worker-owned directory. Killing its cgroup cannot exclude
an outside holder of an already-open writable descriptor: the
[kernel cgroup contract](https://docs.kernel.org/admin-guide/cgroup-v2.html)
limits `cgroup.kill` to the affected subtree. Making the pathname
read-only does not revoke that descriptor on the selected local filesystem.
A terminal process observation therefore cannot establish immutable output.

The operator approved a different ownership model on 2026-10-14: the worker
receives only pipe write ends, while a root-owned collector alone holds the
evidence-file write descriptors in a private directory. This is an execution
contract change, not a throughput proposal. Historical regular-file stream
bindings and signed unit projections remain intact; an enrolled start must use
a distinct pipe-to-collector projection, not reinterpret the old signature.

The first implementation is an executable local writer, with only a nonroot
fixture entry. It creates an exclusive attempt and two new evidence files,
keeps the read ends nonblocking and the worker ends blocking, and pumps at most
8192 bytes per stream per turn. The existing 8 MiB per-stream and 16 MiB combined
limits remain fixed. Overflow keeps a bounded prefix and returns failure, not
a successful receipt with a conveniently truncated report. Neither evidence-file
descriptors nor a production-start capability leave the collector.

Both streams must yield a real zero-length read after queued bytes drain.
The [Linux pipe contract](https://man7.org/linux/man-pages/man7/pipe.7.html)
requires all write-end references to close before read EOF.
Readable hangup, an empty queue and a dead leader are not substitutes. An owned
cat fixture is actually killed with SIGKILL while another holder retains a copy
of its stdout pipe writer. EOF remains absent, and that holder can append late
bytes. Closing the last copy permits drain and the late bytes enter the exact
hash. The test demonstrates why every writer copy, including creator or manager
copies, matters. It does not prove production cgroup ownership or revocation.

The first functional run also rejected a metadata-only assumption. A same-length
file rewrite on the WSL workspace filesystem did not reliably change the
observed timestamps. The collector now rereads the original retained files in
bounded chunks and compares their byte counts and SHA-256 with incremental
stream hashes before acknowledgement. It syncs the files, converts them to
read-only, validates the named objects again, syncs the directory and explicitly
closes its descriptors. This extra I/O must be measured under the existing host
overhead budget later; correctness success is not evidence that it is cheap.

The local fault matrix covers directory and stream creation/sync, append,
read-only conversion/sync, final directory sync, close, acknowledgement and
loss after return. Three exact-source repetitions each interrupt 15 original
collectors with SIGKILL, retain one uninterrupted control and run 32 fresh
refusal inspections. Retained bytes and metadata do not change after the owner
dies. No reopened attempt regains a writer or acknowledgement, even when its
files look complete. Logical I/O-fault injection remains a separate test family;
neither family claims physical power-loss durability.

The [collector contract](../testing/performance/0.74/diagnostic-collector-design.md)
keeps drain data distinct from authenticated completion. The production root
constructor, kernel sender/time admission, independent issuer and authority
pins, exact original-start binding, autonomous watchdog and retained-cgroup
revocation still need connection. Only their joined evidence can publish
completion, advance cells two through four and admit a host lifecycle rehearsal.
No product hot path, frozen workload, estimator, threshold or qualification
input changes in this package.

### Receiving the original sender instead of an asserted peer

The next missing dependency was P2 input provenance. The old portable diagnostic
handler accepts peer IDs and time as caller inputs; using that seam in production
would make the admission decision depend on assertions. Even kernel SO_PEERCRED
is insufficient to identify each message: the
[Unix socket contract](https://man7.org/linux/man-pages/man7/unix.7.html)
describes connection-time credentials, while an inherited socket can be used by
another process later.

The new diagnostic-only receiver consumes the original seqpacket connection
and requires SO_PASSCRED and SO_PASSPIDFD. One recvmsg binds the payload to
credentials and the original message sender's pidfd. It rejects descriptor
passing, unknown controls, duplicates, truncation and invalid size, closing all
received descriptors on refusal. The existing campaign receiver remains intact.
Missing kernel capabilities cause a refusal, not a peercred fallback. The
[kernel SCM implementation](https://raw.githubusercontent.com/torvalds/linux/v6.6/include/net/scm.h)
explains the message PID reference; capability-authorized explicit credentials
still require independently trusted signature/principal admission.

An actual local child inherits a socket whose SO_PEERCRED identifies the parent.
Its packet nevertheless identifies the child and supplies the child's retained
pidfd. After owned SIGKILL that original packet refuses, while a separate parent
packet remains live. This checks original-message identity rather than a numeric
PID reopened after death. Supplementary groups come from the retained original
live process projection, bracketed by pidfd and context checks, not JSON or an
unguarded fresh PID lookup.

Clock input is also produced locally. The receiver retains procfs boot identity
and user/mount/PID/time namespace handles, then samples CLOCK_BOOTTIME for elapsed
lease time and CLOCK_REALTIME only for signature freshness. Boot/namespace drift,
clock reversal and a packet held for 60 seconds refuse. Signed boot identity must
match the observed kernel boot. These facts describe the original receiver
namespace; they do not independently enroll the physical host or supply an
authority epoch.

The package still requires independently selected worker trust and exact
policy digest/epoch pins. The controller and separate builder keys were already
selected in the earlier enrollment; their reuse is described below. No fixture
seed or envelope-supplied verifier can replace that selection. Authenticated
original start, the private root collector,
watchdog/cgroup revocation, completion and cells two through four remain open
integration tasks in the [pre-server checklist](../testing/performance/0.74/pre-server-closure-checklist-20261014.md).
Kernel-input success cannot turn those dependencies into an admitted server run.

### Reusing established trust without inventing worker authority

The controller and distinct builder public keys were already retained in the
[reviewed public policy](../testing/performance/0.74/diagnostic-builder-public-policy.json)
and October 9 enrollment evidence. The raw policy digest matches that earlier
independent selection. The builder private seed was retained only in a protected
GitHub environment secret, while the local SSH key serves only server access.
Neither requires generating a new signing authority to continue local implementation.

The new portable builder loader accepts policy bytes but no caller-selected key
or digest. It delegates canonical parsing and strict signature verification to
the existing verifier under those reviewed pins. A separate kernel packet entry
requires the same reviewed controller key and repository before original sender
and kernel-time authentication. Any key/repository or authentication failure
permanently refuses that packet, including fallback to the generic local method.
The generic model APIs remain unchanged and no production listener is enabled.

Local checks verify the retained real hosted-build receipt signature, reject
changed signatures even with recomputed receipt hashes, and reject policy/key
substitution. Signature validity authenticates that retained statement; without
its executable bytes it does not repeat content verification or grant execution.
These are trust-selection checks, not throughput measurements or proof that
historical keys remain authorized under a current external epoch.

The historical worker-policy evidence uses synthetic issuer keys. A separate
real issuer was subsequently created under explicit operator approval; its
signed host/account policy pin and current authority epoch still need selection.
Trusted installed configuration, controller provenance,
replay/fence joins, original start, watchdog, writer cessation and later cells
remain the boundaries before a server rehearsal. The
[trust contract](../testing/performance/0.74/diagnostic-reviewed-trust-design.md)
keeps these remaining dependencies separate from the public pins already available.

### Creating a separate worker issuer without enrolling a host

The operator selected a separate Ed25519 pair rather than extending the existing
controller key's signing role. A narrow local helper generates it with OpenSSL
into an exclusive directory in the WSL Linux filesystem, outside every Git
checkout. The directory is 0700; the single-link private PKCS8 PEM is 0600.
Trusted no-follow ancestry, retained descriptors and exact named-object checks
refuse links, unsafe permissions, replacement or an existing generation target.
An incomplete operation is retained, not overwritten or repaired by retry.

The private key remains unencrypted in local custody. Permissions do not protect
it from the same account, Linux root or an administrator controlling WSL, and do
not attest whole-disk encryption. No private key is copied to the performance
host or registered as a new CI secret. Only the
[public selection](../testing/performance/0.74/diagnostic-worker-issuer-public.json)
is retained in Git, with SHA-256 of the 32 raw public bytes and a signature over a
fixed selfcheck message in a separate non-policy domain.

OpenSSL accepts that selfcheck and rejects changed message bytes; Rust strict
Ed25519 independently accepts the retained public proof and rejects the
worker-policy domain and controller key. Disposable-key tests cover exclusive
and concurrent creation, unsafe/Git ancestry, symlink/hardlink, exact modes,
special/oversized files, corrupt public selection, key replacement and partial
crypto failure. These checks establish local custody and encoding compatibility,
not authority to execute a workload or a throughput improvement.

The [generation and use note](../testing/performance/0.74/diagnostic-worker-issuer-key.md)
records the exact local location, generation/inspection commands, signing byte
grammar and rotation rules. Policy digest and epoch remain unset. The next trust
join needs an independently admitted host/account policy and current authority,
then authenticated original start, watchdog, writer cessation and completion.
Generating this key closes only issuer selection, not P2 through P5.

### Keeping the selected trust source with the original authority

The next P2 dependency is the source of policy digest and epoch. Letting a signed
envelope, environment variable or another caller choose them would undo the
independent issuer selection. The new input reads only the fixed
`/etc/hydracache-perf/worker-trust-074.json`, with root-owned no-follow ancestry,
a single-link non-executable regular document and a 2048-byte limit. Canonical
schema, fixed repository/issuer and explicit nonzero epoch are checked before use.
No actual production file or policy is populated with model values.

The reader retains the original root, directories, file, metadata and bytes.
Named-object revalidation refuses substitutions rather than reopening a
replacement into the old identity. Full byte comparison remains necessary:
synthetic coarse-stamp/mixed-read cases demonstrate that equal metadata alone
cannot certify the original document. Fragmented reads, interrupted/error reads,
empty/oversized data, links, special files and unsafe modes are separately checked.

Policy verification uses a config/journal/policy/journal/config bracket under
the same active original authority revision/epoch. The returned guard owns the
verified policy and envelope/context, while retaining mutable borrows of the
original config owner and journal. It exposes no raw policy, trust/FD extractor
or start capability. Revalidation cannot replace bytes, host, verifier or epoch;
failures refuse all original runtime owners. Compile-fail tests prevent input
refresh while the guard holds its owners, cross-thread transfer, extraction and
execution. Runtime refusal remains distinct from a durable revocation append.

The first test run rejected an incorrectly nested authority fixture path before
these joins could run. The corrected fixtures use the existing exclusive
`/tmp/hydracache-authority-*` boundary; production restrictions were not relaxed.
Model positives use synthetic issuer keys and nonroot owned trees, never a fake
policy signed by the actual private key. The local public constructor refuses the
nonroot caller before file access. These results establish the reader and original
consistency join, not a positive installed-root or physical-host admission.

The [trust-input design](../testing/performance/0.74/diagnostic-worker-trust-input-design.md)
and [contract](../testing/performance/0.74/diagnostic-worker-trust-input-contract.toml)
preserve the remaining boundary: actual approved policy/body pin/current epoch,
explicit journal enrollment, admitted kernel host/namespace, controller
provenance/replay and original fence must still be connected. The reader neither
initializes an authority journal nor signs or installs a policy. Authenticated
original start, watchdog/writer cessation, completion and later cells stay closed;
no numerical product claim follows from local control-plane safety checks.

### Separate the reusable controller without moving it yet

A reusable test-run controller needs three boundaries: a portable lifecycle and
evidence core, a Linux execution backend with an autonomous supervisor, and the
HydraCache adapter for exact workload identities and qualification policy. The
[boundary note](../testing/performance/0.74/test-run-control-library-boundaries.md)
records this direction without extracting crates or renaming signed artifacts.
Observation and execution remain different capabilities. A library embedded in
the worker cannot supply a surviving watchdog when that worker dies.

Moving the current code before its original-start, whole-writer revocation and
crash gates close would change the structure before validating the complete
ownership protocol. Finish the current lifecycle first, then move one component
at a time against its retained baseline. An independent library still needs
compatibility, failure/drop and overhead evidence; abstraction is not a measured
performance gain or permission to run another workload.

### Compare verified policy with the original kernel context

The separate `verify_kernel_policy` entry removes the caller host argument from
this new read-only path. Candidate machine/boot/user/mount fields come from
bounded signed-policy bytes and are accepted only by the original externally
pinned verifier and active journal. The existing fixed context Probe compares
actual retained machine/boot files and typed reading-thread namespaces. The
original kernel packet independently compares its retained receiver boot/user/
mount objects and keeps sender pidfd/credentials, PID/time namespaces and clock
checks. Equal supplied strings alone cannot satisfy this join.

The returned guard owns the policy/config/journal relationship and Probe while
borrowing the same packet mutably. Later checks bracket context with the original
packet and policy/journal; failures latch all originals, including after wrapper
drop. Replacement arguments, policy/FD extraction and start methods are absent.
Packet freshness keeps its existing exclusive 60-second admission bound; future
consumed-attempt authority must neither renew that request nor mistake this
short-lived observation for an entire campaign lease.

Eight focused runtime/kernel cases pass in three repeats, plus a public API check
and five new compile-fail cases within 68 doctests. Positives use synthetic
issuers/private modeled config and journal alongside real local Linux observations;
they never sign model facts with the actual issuer. Seeded wrong namespaces,
machine/boot mismatches, source/sender/context/policy refusal and restoration
exercise cross-refusal. The registry first caught a missing Linux target; an old
source guard then caught a test-only hook placed before its runtime boundary.
Registering the target and moving only the hook preserve the original assertions.
An incorrect regression-target name is retained separately from functional results.

This establishes actual current-context consistency, not physical initial-host
enrollment, source-opener attestation, real worker account mapping or controller
authorization. Approved real policy/epoch and explicit journal enrollment remain
required. Controller principals/provenance, durable replay/original fence, then
authenticated collector start/watchdog/writer cessation are the next joins. No
server route or numerical measurement is opened by this guard.

### Load controller principals independently of the signed message

Pinning a controller key is only part of principal admission. A caller-supplied
actor/UID/GID allowlist could still select who is accepted. The new fixed source
reads `controller-trust-074.json` under original root-owned no-follow ancestry
and retains metadata and exact bounded bytes. Its separate canonical schema
requires the reviewed key/repository and explicit sorted, unique bounded actor
and nonroot client allowlists. Incoming signatures, worker policy, environment
and selfchecks cannot populate or replace that source.

The signed-request guard retains mutable borrows of that source and the original
worker/kernel chain. Config checks bracket kernel signature/peer/boot/freshness
verification, and revalidation must reproduce the same original request.
Construction or later errors refuse controller, worker policy/config/journal
and packet together. Healthy drop is observation only: reading the same packet
again neither consumes a nonce nor authorizes another executor. A digest getter
does not export request data or a start capability.

Eight functional cases pass in three final repeats, with seeded disallowed actors
and independent bindings. The first fixture restoration tried to rewrite its
own read-only file; restoring test permissions before rewriting fixes that test
without weakening production file checks. Synthetic signers and private nonroot
trees exercise real local kernel peers, not actual installed allowlists or keys.

A valid signature authenticates the selected signer's statement. GitHub run and
actor metadata and the build hash still require independent provenance evidence;
they are not facts supplied by the kernel or this reader. Durable replay and
original fence/early-attempt acknowledgement remain the next ownership join,
after real policy/epoch, principals and host/source enrollment. Only those
producers can lead to authenticated original-start and whole-writer cessation.

### Bind the builder statement to the original controller request

Independent builder key selection did not previously force its receipt to remain
joined to the live original controller/kernel/worker input. The new read-only
binding takes the public builder policy and receipt, verifies them under the
fixed reviewed pins against the identity in that original request, and keeps
the original controller guard mutably borrowed. It owns bounded copies of the
policy and receipt along with the checked builder authority and statement.
Original-context checks bracket verification; later checks accept no replacements
and require the same identity and statement. Any error refuses the entire
original config/policy/journal/packet chain rather than allowing a new receipt to
repair that observation.

Eight focused cases pass in three repeats. They combine a retained real historical
builder signature with synthetic controller/worker signers, private nonroot
config/journal trees and real local Linux messages. Thirty-two seeded signature
mutations still refuse when the synthetic controller signs their recomputed
receipt hash. Tests also cover wrong original identity, budgets, original owner
and retained-byte drift, permanent refusal, caller-buffer changes and healthy
drop. Five new compile-fail cases prohibit extraction, start, replacement checks,
thread transfer and refreshing an owner while its binding remains borrowed.

The first test compilation needed an explicit checked conversion from the
receipt's u64 budget to a vector length. A regression command named a nonexistent
builder integration target; the corrected artifacts target includes those builder
checks. Both development failures remain in the local evidence packet. No
production validation was relaxed to make either check pass.

This authenticates the builder's signed statement, not an installed executable
or an independently observed compilation. The existing receipt has no controller
run ID, attempt, actor or workflow identity. Its signature therefore cannot prove
the GitHub history asserted by the controller. That distinct provenance input,
durable nonce consumption and common original fence remain prerequisites before
authenticated start, watchdog and whole-writer cessation. No wire schema, real
policy/epoch, selected key, frozen qualification input or server entry changes.

### Select an independent source for controller origin

The workflow audit exposes a protocol distinction, not a missing field to copy.
The current qualification job signs campaign start/attach/seal/abort with a
UUID request ID and 300-second authorization; diagnostic requests use a distinct
domain, nonce, lease identity and at most 60 seconds. The separate builder
receipt authenticates a fixed build statement, but its signed fields do not
include controller run or workflow identity. Artifact names and unsigned
environment observations cannot bridge these boundaries.

The [proposed origin decision](../testing/performance/0.74/diagnostic-controller-provenance-decision.md)
recommends retaining the controller signature and adding independently verified
GitHub OIDC evidence. GitHub supports job-origin claims and custom audience
selection; see its [OIDC reference](https://docs.github.com/en/actions/reference/security/oidc).
Our proposed audience would bind the same original request digest, with separate
reviewed workflow and verification-key pins. Unknown/stale keys and origin,
request or time mismatches would refuse rather than select downloaded trust.
The operator approved local implementation of this additional trust architecture,
not real issuer enrollment or a successful server gate. Public-key custody/rotation and
the dedicated diagnostic signing job still require review. Even after origin
verification, durable replay/common fence and original-start/writer revocation
must establish their own authority; a token does not consume a request nonce.

### Verify configured OIDC origin without enrolling a production issuer

The first approved implementation verifies RS256 against one explicitly selected
RSA key in a separate fixed root-owned trust file. It uses already locked ring
and base64 versions; only their two direct Linux supervisor dependency edges
change. The verifier checks the original compact JWS signing bytes, strict
header and bounded unique scalar claims. Pretty-printed claims can remain valid
signed bytes; reserializing JSON before verification would verify the wrong input.
Duplicate names, including escaped equivalents, wrong key/algorithm, token key
URLs, invalid encoding and origin/request substitutions refuse.

The audience derives from the original diagnostic request digest. Required
repository/run/attempt/actor, subject, workflow revision, dedicated environment,
event and hosted-runner fields must match the separately configured profile.
Original kernel time gates token/snapshot validity without renewing request or
packet freshness. Actor ID describes the initiating account, not independent
approval by a rerun operator; protected-job approval remains a separate producer
requirement. This profile deliberately excludes reusable-workflow claims until
their policy is reviewed.

The returned read-only guard retains mutable ownership of OIDC configuration and
the original controller/build/worker/kernel chain, with bounded private token
bytes. Revalidation cannot replace trust or token, and failures cross-latch all
originals. It exposes neither claims nor a start capability. Synthetic RSA and
controller/worker keys exercise actual local Linux packets; none are selected
as production authorities. A publicly committed synthetic PKCS8 fixture is test
data, never a credential for a real issuer or deployment.

Ten focused cases, including the original kernel-clock sampler, pass in three
repeats, alongside five new compile-fail boundaries. The first negative mutation
mistakenly expected any changed nonempty jti to refuse. Replacing it with an empty
jti tests the real bounded syntax contract without inventing a token allowlist.
The initial root check also found a formatting-sensitive source marker and an
old evidence check comparing frozen observer locks to current controller tooling.
Frozen lock hashes remain unchanged and are checked against both exact original
Git sources; a separate current-lock guard allows only the two approved dependency
edges, with every package version/checksum and other dependency unchanged.
Synthetic builder fixtures also retain the observer's exact frozen lock bytes
and require its existing SHA instead of using the newer controller lock.

This establishes configured cryptographic observation, not authenticated GitHub
key acquisition, current key-review/rotation policy, an actual protected-job
receipt or durable replay/common fence. Those producers remain before original
start, collector/watchdog/writer cessation and later-cell completion. Existing
wire/durable artifacts and frozen qualification inputs are not amended.

### Produce the exact diagnostic request separately from admission

The job-side producer now validates one canonical request against the reviewed
builder policy and receipt before touching a signing key. Its audience is the
same domain-separated digest required by the OIDC verifier. Signing consumes
that preparation, requires the already selected controller key, self-verifies
the original diagnostic signature and checks job UTC before and after signing.
It returns signed bytes, not a receiver guard, consumed nonce or start authority.
Recreating preparation remains possible; durable replay belongs to original
receiver admission, not this stateless tool.

A separate Linux CLI reuses protected original-file readers and exclusive synced
output. Private directory/leaf modes, single-link files, key-purpose matching,
bounded inputs and refusal without echoing credentials are covered by local
tests. Successful signing uses only synthetic private test selection; the public
entry refuses that key. The installed supervisor and campaign workflow acquire
no new route or permission. Compile failures in new tests exposed a missing
metadata trait, private helper imports and a SHA-array formatting assumption;
the fixes stayed in the tests rather than expanding the public API.

The public-source inspector is deliberately offline. It checks discovery/JWKS
syntax, exact endpoints and public RSA bounds, then reports hashes and metadata
with acquisition, selection and admission false. Certificate fields do not
verify a chain, and an unsigned local file does not become trusted through
parsing. This separates a repeatable parser test from independently authenticated
key acquisition, custody and rotation. The Python builder regression also now
reconstructs exact frozen observer fixture bytes and verifies their original
digest instead of comparing them with the newer controller tooling lock.

The next integration must preserve one body across audience derivation, token
retrieval, signing and receiver verification. Actual protected workflow/pins,
issuer acquisition and original replay/fence remain prerequisites for start;
producer success cannot stand in for any of them. See the
[local producer procedure](../testing/performance/0.74/diagnostic-controller-producer-operator.md).

### Bound public key acquisition without selecting production trust

The separate public acquisition tool now requests only the fixed discovery and
JWKS paths. TLS requires the fixed hostname, SAN verification, a strict full
chain under platform CA roots and TLS 1.2 or newer. Redirects, compression,
ambiguous framing, duplicate headers, over-budget bodies and inherited proxy,
CA/key-log/OpenSSL/Python or known bearer-token overrides refuse. It neither
reads a signing key nor accepts a token or token-selected URL.

Socket timeouts alone do not bound a slow DNS resolver or headers delivered one
byte at a time. An isolated owned child therefore receives a minimal environment
and a total subprocess deadline. Timeout kills and reaps only that child and
refuses partial output. The parent checks schema, raw public source digests,
transport metadata and exact inspector types; JSON zero is not boolean false.
Raw sources and platform CA/peer certificate hashes remain inspectable without
turning a receive timestamp into an authority epoch or snapshot expiry.

Real loopback TLS verifies a synthetic positive, unknown-CA refusal and wrong
hostname refusal. The first Windows run exposed a fixture assumption: the peer
can reset TCP instead of delivering a TLS alert. The client still had to raise
certificate verification failure; only the fixture's handshake observation was
made portable. An owned blocking child additionally proves timeout reaping and
partial-output refusal. Modeled HTTP/report mutations and independent acquisitions
exercise the remaining paths without contacting the actual issuer. The ordinary
CI step runs these local fixtures, not the acquisition command or qualification.

This adds a transport mechanism, not a production-key enrollment receipt. Platform
CA/source/interpreter custody, actual key acquisition and explicit key/workflow/
expiry review remain before a protected diagnostic job. A copied unsigned report
cannot authorize replay, original start or completion. The
[acquisition procedure](../testing/performance/0.74/diagnostic-controller-producer-operator.md#public-acquisition-tool-after-separate-approval)
keeps those choices separate from the local parser and TLS evidence.

### Observe the real public source without choosing a signing policy

The next step received the fixed GitHub discovery and JWKS once over TLS 1.3.
The initial inherited environment failed the existing guard because a CA-bundle
override name was present. A separately owned clean process used platform roots;
no override value, parent setting or CA store was changed. Runtime/source and CA
inventory match before and after. The byte-exact public bodies, TLS peer digest
and native outcome are retained, and independent offline inspection reproduces
the four-key roster and its source hashes. This is real public-source observation
under that platform trust, not a modeled GitHub response or a protected-job token.

The roster exposes a practical selection issue: the current v1 verifier accepts
one configured key, but the public endpoint supplies four. Selecting the first
key would not identify the signer of a future token. A separately reviewed
bounded trusted roster would support rotation without permitting token-controlled
URLs or embedded keys; that is a schema/policy decision, not an automatic
promotion of received data. Production key/workflow/subject/expiry remain unset.

Protected-job preparation also separates build and signing authority. Job-wide
token permission cannot be treated as a late step-level grant, so compiling in
the token-bearing job is not credential-free bootstrap. The signing job needs
independently authenticated prebuilt tooling; the frozen observer builder receipt
does not attest that controller executable. Body retention across audience,
token retrieval and signing, bounded credential transport and original
receiver/replay/fence remain code and custody work. No active workflow or token
was created to make the observation look like execution admission.

The [public-source packet](../testing/performance/0.74/local-runs/diagnostic-oidc-public-source-20261014/README.md)
and [job preparation](../testing/performance/0.74/diagnostic-controller-job-preparation.md)
keep the successful network read, selection decision and remaining original-start
prerequisites distinct.

### Support issuer rotation without token controlled trust

The approved local roster extension resolves the schema issue found in the real
public observation. The new v2 configuration accepts one through eight keys in
8192 canonical LF bytes; v1 keeps one key and its 2048-byte document limit.
Worker/controller configurations keep their original source budget. Every key
passes the same RSA/RS256 constraints, kids are strictly ordered, and duplicate
modulus/exponent pairs refuse even when their names differ.

The hypothesis is operational rather than a throughput claim: several explicitly
selected issuer keys can support rotation without allowing an incoming token to
choose new trust. Local synthetic tokens exercise each selected position and a
deliberately wrong known kid whose signature matches another member. That latter
case must fail, not succeed by trying every key. Unknown kids and expired
snapshots still refuse without downloading or falling back. Replacing a valid
snapshot also refuses the retained original controller/worker chain; restoring
the old bytes cannot repair that runtime guard.

One shared verifier retains the original signing input, strict claims, audience,
origin and exclusive windows for both versions. No production snapshot was
installed and no real token was requested. The
[roster design](../testing/performance/0.74/diagnostic-oidc-roster-design.md)
and [local evidence](../testing/performance/0.74/local-runs/diagnostic-oidc-roster-20261014/README.md)
keep reader support distinct from actual issuer/job
enrollment. The next producer owners are authenticated prebuilt-signer custody
and retaining one immutable request through audience, token retrieval and signing.

### Retain the request before coordinating credentials

The next producer change isolates body consistency from credential authority.
Two standalone CLI invocations can read different public files even when their
filenames match. The new local signing API instead consumes the earlier opaque
preparation, recomputes its canonical body digest and verified builder identity,
and checks its stored and observed audience before opening a signing key. It
takes no replacement request, builder-policy or receipt path.

The hypothesis is directly falsifiable: alter or remove every public input after
preparation and the synthetic signature must still bind the original body and
audience. Pair a different valid preparation with the earlier audience and it
must refuse before even opening an absent secret file. Tests exercise both,
plus expiry, malformed audiences, selected-key purpose, exclusive output and
seeded caller-buffer replacement. The existing key metadata, clock brackets and
signed wire domain remain; ownership consumption does not become durable replay
protection. The old file CLI delegates internally but its separate commands still
reread files.

This closes the retained file-signing seam, not actual token retrieval or origin
verification. A future trusted coordinator must hold that same object through
the real token exchange and validation. Bootstrap is independent: the frozen
observer builder receipt does not attest the signer executable, and neither a
self-reported executable hash nor the controller key can supply independent
custody for code reading that controller key. No key-purpose expansion or
production artifact/pin selection is made by the
[retained producer](../testing/performance/0.74/diagnostic-retained-producer-design.md).
The [local packet](../testing/performance/0.74/local-runs/diagnostic-retained-producer-20261014/README.md)
retains repeatable synthetic body/signature results and unchanged original
qualification inputs without claiming production origin or a performance gain.

### Separate signer assertions from observer builder authority

A build receipt is meaningful only within its issuer's reviewed purpose. The
existing builder attests one frozen observer binary; reusing that authority for
code that reads the controller private key would silently enlarge the trust
boundary. The controller key cannot independently attest its own signer either.
The next approved local step therefore introduces a dedicated signer-build schema
and signature domain, and refuses the known controller, observer builder and
worker issuer keys for the new role. No production issuer is selected.

The hypothesis is narrower than bootstrap authentication: a separate signed
statement can bind a source/toolchain/profile and all five supplied artifacts
without becoming an old observer receipt or execution permission. The model
requires an exact expected statement and SHA256/length matches for binary,
lockfile, build log, runtime manifest and bootstrap context. Strict canonical LF
JSON, bounded inputs, unknown/duplicate/future refusal and domain-separated
Ed25519 verification prevent alternate shapes and cross-purpose reuse.

Nine synthetic integration cases test every statement field, all signature
positions, wrong issuer/domain, old key-role reuse, malformed JSON and each
artifact substitution. Seeded mutations and independent concurrent models check
that successful verification cannot hide a later mismatched input. The retained
observer receipt still verifies in its own reader and refuses in the new one;
the old reader also refuses the new signer shape. Compile-fail tests prevent
turning the result into a verified observer build, cloning it or deserializing it.

The positive deliberately uses `not ELF` and opaque runtime/context bytes. This
is not a shortcut in a production loader: the new API has no loader or authority
consumer at all. It makes the precise result visible. A valid signature authenticates
an assertion under a supplied key; matching content authenticates neither that
key's selection, compilation nor the original mapped executable. Its result
exposes only a diagnostic receipt digest. Promoting it directly into permission
to read secrets would defeat the separation the schema establishes.

The [model design](../testing/performance/0.74/diagnostic-signer-build-design.md)
and [local packet](../testing/performance/0.74/local-runs/diagnostic-signer-build-20261014/README.md)
retain the positive and refusal checks. Independently selected production issuer
and pins, actual prebuilt-artifact acquisition, original executable/runtime
custody and pre-exposure bootstrap remain the next owners. Those must be joined
to the retained request through real token coordination before original
receiver/replay/fence/start and all-writer cessation can open server execution.
This step changes local verification discipline, not measured cache throughput.

### Retain the file object rather than trusting its name

Once a separate signer statement exists, the next seam is the filesystem input.
A receipt and a pathname can continue to look identical while the named object
has changed. The local Linux reader now holds the directory and six original
file descriptors, walks components with read-only no-follow openat, and refuses
unexpected names, unsafe metadata or nonregular/multilink children. The selection
and directory owners remain explicitly asserted; this is not production enrollment.

The hypothesis is about object identity rather than speed: rechecking a bundle
must refuse an equal-byte replacement inode, not merely a different hash. Each
recheck compares named files with original descriptors, streams original hashes,
and brackets dev/inode, owner/mode/link/length and mtime/ctime plus directory
membership. These metadata timestamps detect observed changes; they do not
supply authority ordering. Hashing uses a fixed 64 KiB scratch buffer, and only
the bounded receipt gets a temporary byte copy. Large binaries and logs do not
become retained payload buffers.

Tests perform actual replacements, in-place writes, restored names/bytes,
symlinks, hardlinks, FIFOs, extra/missing entries and sparse over-budget files
inside owned nonroot temporary directories. A private child check isolates
equal-byte inode refusal even when the directory stamp also changed. It does
not bypass the public guard. Seeded substitutions and independent concurrent
guards check sticky refusal; CLOEXEC and post-drop inode observations check the
original descriptor ownership. Chunk-boundary hashing and compile-fail/API checks
cover read ceilings, empty inputs and the absence of FD or replacement exports.
The first lint pass found two test-style errors, which were corrected without
weakening lint or runtime predicates.

This closes an original file-input seam, not the signer bootstrap. A successful
check describes an observed interval and cannot make a later pathname exec
atomic. Runtime/bootstrap manifests still contain opaque signed assertions;
the reader has not identified a mapped executable, loader or shared libraries,
authenticated trusted ancestry, selected production pins or acquired job custody.
It returns inventory only and has no producer/CLI/start consumer. The
[file design](../testing/performance/0.74/diagnostic-signer-bundle-design.md)
and [local evidence](../testing/performance/0.74/local-runs/diagnostic-signer-bundle-20261014/README.md)
keep the real descriptor proof separate from those remaining pre-exposure
bootstrap requirements. No credential, server launch or throughput claim is
authorized by the synthetic receipt or its original file handles.

### An executable inode is not the original invocation

The next local step connects the original file-input owner to an existing
retained process observation. It owns a read-only clone of the expected binary
and an O_PATH/CLOEXEC handle for the original procfs executable target. Rechecks
compare original and fresh target metadata while bracketing the retained
pidfd/generation/boot/group/cgroup and bundle hashes. A different executable
inode refuses even if PID and start ticks have not changed; failure stays
latched in the wrapper and bundle after drop. Healthy drop changes neither
original owner and grants no permission to signal, sign or start.

The hypothesis has a useful counterexample. An owned copied shell first
acknowledges readiness, then execs a different `/bin/cat` object. The retained
generation observation remains valid while executable binding refuses. That
proves why process generation alone is insufficient for executable identity.
Another controlled shell re-execs the very same copied binary. It acknowledges
readiness a second time, yet the generation and executable object still match.
The new check therefore cannot prove the original invocation or the absence of
intervening same-inode exec. Treating this accepted observation as authenticated
original-start evidence would be a false conclusion.

Eight wrapper tests, an internal expected-descriptor case and the borrowed API/
compile-fail checks exercise original positives, equal-byte foreign inode,
wrong process, different exec, same-inode counterexample, exit, bundle drift,
sticky refusal, healthy drop and descriptor ownership. A fixed seed adds fresh
controlled observations and independently owned helpers. Their environment is
cleared and the fixed protocol uses only acknowledgements, exec and EOF exit;
no product/service/cgroup mutation or signal is part of this step. Legacy stream
binding retains its predicates and remains separately tested.

The result is a kernel executable object observation, not a signature over the
process's loaded instructions. Loader, shared libraries, interpreter, JIT,
preloads and bootstrap environment remain outside the check. Production issuer,
pins, ancestry and job custody are still unselected, and no credential or start
consumer accepts this borrowed guard. The
[executable design](../testing/performance/0.74/diagnostic-signer-executable-design.md)
and [local evidence](../testing/performance/0.74/local-runs/diagnostic-signer-executable-20261014/README.md)
preserve the counterexample alongside the useful refusal proof. The next
pre-exposure work must establish runtime/bootstrap custody and original
invocation evidence, rather than broaden the meaning of an inode match.

### Original launch needs a separate trust boundary

The same-inode counterexample determines the next engineering decision. Adding
more metadata to the existing file/process check would still be observation
after the fact. Original invocation instead requires a launcher that owns the
child from creation, accounts for the admitted runtime, observes the first exec
and controls the handoff before secrets can reach that child. The current
asserted readers must not manufacture that launch authority.

The [proposed bootstrap boundary](../testing/performance/0.74/diagnostic-signer-bootstrap-decision.md)
preserves the existing GNU release profile. A static/musl rebuild might simplify
runtime inventory but would introduce a new build contract and would not solve
original invocation by itself. The proposed local prototype evaluates kernel
exec events, denial of later exec through both syscall forms, and a private
one-shot dummy handoff. It must account for threads, alternate ABI, runtime
inputs and owner loss; a ready message or a seccomp flag alone is insufficient.
The trusted kernel, launcher administration and reviewed signer remain explicit
assumptions, not memory-attestation claims. The operator approved the local-only
prototype before its first implementation slice.

Job authority is a different boundary again. GitHub's OIDC permission is scoped
to a job or workflow, so moving token retrieval to a later step does not remove
earlier steps from the trusted job. Build and dependency installation must
remain in a job without that authority, while the token-bearing job needs its
own reviewed bootstrap and runtime before it starts. The
[official permission reference](https://docs.github.com/en/actions/reference/security/oidc#workflow-permissions-for-the-requesting-the-oidc-token)
describes that scope. A local child handoff cannot retroactively isolate an
untrusted job that already has token-request permission.

The [decision audit](../testing/performance/0.74/local-runs/diagnostic-signer-bootstrap-decision-20261014/README.md)
reproduces all eight executable cases at the unchanged source, including the
same-inode observation. This establishes the reason for the proposal, not a
successful launcher or credential grant. Worker original-start, writer
revocation, later cells and server rehearsal still need their own proofs.

### Kernel exec stops expose an inherited descriptor race

The first approved slice tests one hypothesis: an owner created before exec can
distinguish a new invocation even when the executable inode stays unchanged.
The private Linux x86_64 fixture forks its own child, retains the original pidfd,
descriptor-execs the selected binary, and requires a PTRACE_EVENT_EXEC stop
before continuing it. The helper output is still empty at that stop. A later
same-inode exec generates another kernel event, which refuses the owner and
terminates the owned helper before the new image runs user code. This addresses
the previous observational counterexample without redefining inode equality.

Concurrent runs then revealed a separate implementation fault: occasional exec
errno 26, ETXTBSY. CLOEXEC did not close an inherited writable descriptor while
another helper remained stopped before exec. A deterministic two-child test
reproduced it: open a writable descriptor for the second binary, fork and stop
the first child, close the parent's writer, then attempt the second exec. The
first stopped child still held the writer and blocked execution. This explains
the observed fault without selecting a timing threshold or adding retries.

The fix closes every nonselected descriptor above stdio in the child immediately
after redirection, preserving only the selected executable. The same two-child
regression then passes while the first child remains stopped. The closure
affects the forked child's copied descriptor table, not the parent's table;
an unsupported syscall refuses instead of falling back. This is both a
reliability fix and an inherited-descriptor boundary. It is not global writer
revocation or a production sandbox.

The [first-exec packet](../testing/performance/0.74/local-runs/diagnostic-signer-first-exec-20261014/README.md)
retains the original failures, errno diagnosis and expected-red regression,
followed by three eleven-case repeats, seeded fresh launches and independent
tracer threads. Strict checks and a normal-library symbol inspection keep this
launcher confined to test builds. The bounded ELF identification intentionally
accepts a truncated header whose actual kernel exec fails; it does not attest
the runtime. That first slice does not implement exact filter installation or
credential handoff. The next slice below checks filter installation separately;
their composition, actual tracer loss and independently selected runtime custody
still precede any signing or worker-start consumer.

### Exact filter installation needs original memory and kernel results

The next hypothesis is narrower than a working signer sandbox: can the owner
prove which fixed filter its own helper actually asks the kernel to install,
then observe the intended syscall denials? A ready message or a Seccomp=2 flag
does not identify the filter. Privileged filter extraction would add a different
host boundary, so the local fixture instead inspects genuine syscall information
and bounded original memory while the owned child is stopped.

The private helper is forked from the current test image, with its header and
BPF array prepared beforehand. At the installation entry, the owner requires
the exact native seccomp operation, TSYNC flags and original header address.
It checks header length, original program pointer and every encoded instruction,
then requires a zero, non-error kernel exit and checks those bytes again. An
equal-byte replacement header or program pointer refuses just like altered
bytes. This is a sequential observation under a trusted creator/kernel and no
external memory writer, not an atomic signature over executable memory.

Two development failures shaped the implementation. The first full run passed
seven tests but failed the alternate-ABI case because the kernel reported a
syscall-exit stop before terminal death. A focused capture exposed the exact
x32 exit shape. The observer now accepts only that architecture/number-valued,
non-error exit followed by SIGSYS, or direct SIGSYS; it does not treat an
arbitrary intermediate stop as success. The terminal predicate remains intact.

Fatal probes also require care outside the filter itself. The local host uses
a piped core collector, where RLIMIT_CORE=0 does not suppress collection. The
[core manual](https://man7.org/linux/man-pages/man5/core.5.html) describes this
exception. Before final tests, the fixed policy was amended to allow only
PR_SET_DUMPABLE/zero, and the owner must observe its successful kernel entry/exit
after checking installed memory and before any fatal probe. Restoration of
dumpability is denied. No global collector setting or credential changed.
An initial ordering mistake in that integration failed five tests and is
retained alongside the correction, rather than counted as a successful run.

Three final eight-case repeats pass. Real native execve/execveat attempts,
including the inherited test image and a different executable, return EPERM;
fixed task-creation/control calls refuse, while real x32 and i386 int80 probes
end with SIGSYS. Policy interpretation covers 1024 bounded syscall numbers and
bad architectures, strict capture decoding and instruction mutations. Seeded
fresh helpers and independent tracer threads exercise ownership without making
the child multithreaded. The
[seccomp design](../testing/performance/0.74/diagnostic-signer-seccomp-design.md)
and [local evidence](../testing/performance/0.74/local-runs/diagnostic-signer-seccomp-20261014/README.md)
bind those results to the exact source and preserve the failures.

The useful result is exact installation and kernel denial on a single-thread
owned helper. TSYNC success here does not prove support for an already
multithreaded GNU signer, and allowing other native syscalls makes this an
exec/task-control policy rather than a complete sandbox. First exec, runtime
custody and installation still need one original-owner chain before dummy
handoff and real owner-loss tests. That next composition must not convert the
older asserted observation guards into authority. Worker start, all-writer
cessation and later lifecycle cells remain separate work; this correctness
slice adds no allocation, throughput or release-readiness claim.

### A fresh executable must carry the installation proof

The next local hypothesis joins the two mechanics instead of assuming that
success on separate helpers composes. A small fixed Rust helper is compiled
before launch with the existing GNU target and release-like O3/panic-abort
settings, without external crates or credentials. One owner creates its child,
descriptor-execs that image, requires the exact kernel exec stop, follows GNU
startup and then observes the exact filter installation in the new address space.
The same child must subsequently receive EPERM for self/different execve and
descriptor execveat before exiting successfully. No ready text supplies authority.

Fresh exec changes the address space, so the earlier parent-preselected stack
pointers cannot simply be carried across. The new observer captures the actual
installation header once from kernel syscall information, bounds it and its
program to the original stack mapping, freezes both addresses and checks all
policy bytes again after exact zero exit. This proves actual kernel inputs for
the trusted fixed helper, not independent provenance of an arbitrary pointer
or protection against an external memory writer. Before installation, another
exec, task creation, control calls and unsupported syscall-number forms refuse
at entry; observed task/descriptor checks remain distinct from runtime custody.

The first development problems were ordinary preparation errors: the pinned
digest type did not support the selected hex formatter, and cleared environment
lookup could no longer find rustup through the previous PATH. Explicit byte
formatting and local compiler discovery fixed them without changing dependencies
or restoring inherited environment. Those failures remain in the evidence.

A repeated run then exposed a stronger version of the inherited-writer problem.
Closing descriptors before the initial stop is not enough if a different fork
has inherited an image writer but has not yet reached that closure. The parent
may finish writing and release its FD while that in-flight child still holds
the writable description. The original rare failure had only a generic exit
status; a deterministic stopped descriptor-table snapshot subsequently reproduced
the mechanism with actual ETXTBSY. The distinction prevents hindsight from
turning an uncaptured errno into a measured fact.

The fix removes that description from the parent's table: an isolated local
copy process materializes the image while the parent holds only a pipe. The
parent closes input, waits for success, verifies the bytes and freezes the image
before original launch. A stopped snapshot no longer blocks exec. Deliberately
giving a snapshot an external image writer still produces ETXTBSY, so this is
owned preparation isolation, not a claim that every writer has been revoked.
No retry, scheduling threshold or serialized benchmark hides the counterexample.

Three nine-case repeats pass, including phase/replay and memory-range refusals,
installer faults, metadata substitution, seeded fresh children and independent
tracer threads. All three builds produce the same helper hash locally. Real
SIGKILL at first-exec and installed boundaries leaves a refused/reaped original
child with no handoff; actual tracer-loss EXITKILL is still a different test.
The [joined design](../testing/performance/0.74/diagnostic-signer-joined-design.md)
and [evidence](../testing/performance/0.74/local-runs/diagnostic-signer-joined-20261015/README.md)
retain the unsuccessful repeat and deterministic red alongside final-source
success. Normal libraries expose no launcher symbols or production consumer.

This joins original exec and installation mechanics, not the remaining trust
inputs. The system GNU loader and libraries are still fixture dependencies;
their independent selection/custody must precede one-shot dummy handoff. A
single-thread child does not establish multithreaded signer support, and local
child death does not prove autonomous owner-loss cleanup. Production job/issuer
custody, worker start and all-writer cessation remain separate. The result
strengthens the correctness of the test apparatus without changing cache hot
paths or claiming new allocation or throughput gains.

### Declared runtime inputs are not the runtime that executes

The joined helper still used the host GNU loader and libraries. The next local
hypothesis asks whether we can enumerate that fixed helper's declared ELF
dependencies and retain exactly the original files without mistaking the result
for independent runtime custody. A GNU target or executable hash alone does not
answer which loader, libraries or configuration a launch will actually use.

The implementation is a private Linux x86_64 test module with a bounded byte
parser, not ldd or a generic launch API. It checks ELF64 header/program ranges,
LOAD interval/alignment and entry constraints, non-WX segments, nonexecutable
stack, one DYNAMIC segment and the fixed interpreter. DT_STRTAB is translated
from a virtual address through its file-backed LOAD. Treating it as a file
offset would parse unrelated bytes. The [ELF manual](https://man7.org/linux/man-pages/man5/elf.5.html)
documents those distinct fields. Unsupported forms, search paths, ORIGIN,
TEXTREL, audit/filter/auxiliary tags and unsafe names refuse within fixed bounds.
Known relocation/symbol/version metadata is not fully validated; section tables
are ignored. Accepted synthetic metadata is not proof an ELF can safely execute.

The flat fixture contains exactly the signer, GNU loader, libc and libgcc.
Its declared edges must resolve to selected SONAMEs with no foreign, missing,
unused or self-dependent object. Original readonly NOFOLLOW/CLOEXEC descriptors
remain held alongside hashes and metadata. Rechecks compare both original and
fresh named identities, content and parsed declarations under root brackets.
The first mismatch stays refused even after bytes are restored. FIFO, symlink,
hardlink, directory, wrong identity/mode, extra entries, equal-byte replacement
and content drift cannot repair acceptance. Direct synthetic metadata resets in
tests isolate inode and digest guards; there is no owner refresh API.

Three ten-case repeats pass, including seed 0x07403b28, 128 no-panic parser
mutations and four independent owners. The same helper and three library hashes
are observed in each repeat. The mutations do not claim every changed byte must
refuse, and independent owners do not make a signer multithreaded. Rust 1.88
parent-crate compatibility, current strict checks, existing launch/filter/file/
producer regressions and ordinary-library symbol exclusion pass. A missing-owner
compile failure remains in evidence. An early Rust 1.94 diagnostic-renderer ICE
while processing unused test code is also retained; completing the tests removed
that diagnostic path without suppressing warnings or changing the toolchain.
An incorrect regression command is recorded separately from corrected results.

The useful result is bounded declared dependency inventory and point-in-time
original-file consistency. It is not an atomic immutable snapshot or cessation
of every writer. Copying four local images does not force the kernel and loader
to select those copies. Cache/configuration, preload, hwcaps and later dlopen
remain relevant; see the [loader manual](https://man7.org/linux/man-pages/man8/ld.so.8.html).
Local system sources and fixture UID/GID are setup assertions, not enrolled
production ancestry. No inspected image is executed by this module, and no FD
or success bit feeds a signer/token/worker-start consumer.

The [runtime design](../testing/performance/0.74/diagnostic-signer-runtime-design.md)
and [evidence](../testing/performance/0.74/local-runs/diagnostic-signer-runtime-20261015/README.md)
bind source, raw captures and unchanged qualification inputs. Next enforce the
selected loader/root and configuration and compose them with the original
launch. Independent production custody, one-shot dummy handoff and actual tracer
loss still need distinct proofs. Worker P2/P3, all-writer cessation, later cells
and server rehearsal remain separate. These tests measure no cache speed,
allocations or product workload and add no release-readiness claim.

### A selected path root still needs an original executable

Library-path priority does not exclude fallback into host paths. Directly
invoking a selected loader has another consequence: the first kernel executable
object is the loader, not the signer. Those distinctions matter when joining
runtime selection to an original-exec proof. The
[loader manual](https://man7.org/linux/man-pages/man8/ld.so.8.html) describes direct
invocation and the search order. The next experiment therefore checks a fixed
root before adding any new exec path.

The private fixture creates four readonly files at the original GNU interpreter
and library paths beneath an owned temporary root. Its parent holds original
descriptors, hashes and exact tree identities, but no writable image description:
isolated copy children preserve the previous writer-inheritance fix. One owned
raw-fork child closes nonselected FDs, stops under its original tracer and
creates a user namespace. The owner supplies single-entry maps preserving the
same nonzero UID/GID. Namespace capabilities permit the temporary root selection
without granting host-root authority; their scope is explained in the
[user namespace manual](https://man7.org/linux/man-pages/man7/user_namespaces.7.html).

The child enters the original root, resets cwd to `/`, closes its directory FD,
drops all capability sets and sets no_new_privs. Before permitting the fixed
probes, the parent checks kernel root/cwd and namespace objects, exact maps,
UID/GID slots, capability status, one task and the selected descriptor set.
Absolute signer/loader/library paths then resolve to the original fixture inode
pairs. Checked host passwd/proc/cache/preload paths return ENOENT, and `/..`
remains the selected root. The parent root and namespace stay unchanged.

The first kernel positive passes. Strict lint then rejects the attempt to include
the ELF source as two separate modules. Sharing one pure four-image validator
fixes that code organization without a lint waiver or exported guard/FD. A
separate review catches unsafe cleanup through replaced ancestors; cleanup now
uses original directory descriptors, and a symlink-substitution test confirms
that a different owned directory is not chmodded. An incorrectly transported
shell filter also starts the wider unit suite; that local group is interrupted,
retained as incomplete and replaced with explicit focused commands. None of
those development captures is counted as a successful final gate.

Three seven-case repeats pass, with seed 0x07403b29, eight fresh path probes and
four independent owners. Actual SIGKILL at initial and rooted stops leads to
refusal and exact-child reaping. A root permits one child attempt only; phase,
namespace, mapping, capability, tree, file and digest faults cannot manufacture
a replacement. Current/MSRV checks, existing regressions and normal-library
exclusion pass. The [design](../testing/performance/0.74/diagnostic-signer-root-design.md)
and [local evidence](../testing/performance/0.74/local-runs/diagnostic-signer-root-20261015/README.md)
retain the failed steps and bind final results to their source.

This establishes path-root mechanics on this local kernel, not an admitted GNU
runtime. The raw-fork probe still has the test image's old mappings and executes
no selected ELF. Chroot is not a complete sandbox, as the
[chroot manual](https://man7.org/linux/man-pages/man2/chroot.2.html) cautions; zero
current capabilities also do not prohibit arbitrary later namespace operations.
Next join original descriptor exec, selected interpreter/library objects and
exact installation, then constrain later root/namespace changes before dummy
handoff. Independent job/runtime custody, actual tracer loss, worker start and
all-writer cessation remain separate. No cache-performance result follows from
this test-apparatus change.

## Joining the selected root, original exec and installation (2026-10-15)

The next hypothesis concerned composition rather than speed: would the original
GNU helper still start from the retained root and reach the exact existing filter,
without the launcher quietly relying on host loader paths? Separate root probes
and separate exec/filter tests could not answer that. A successful path probe
does not show which objects an actual loader maps, and a successful host exec
does not show that the selected root participates in that invocation.

The new private test owner therefore creates its own root and child, retains the
original pidfd and observes the complete local chain. Child-only user maps/root,
capability removal and no_new_privs precede original descriptor exec. At the
kernel exec event, the original signer, root/cwd, namespace, credentials, single
task, empty environment and closed input FDs must match. At first exec only
signer/loader file identities may appear; at installation entry and zero exit,
all four selected images must appear. Kernel-reported device/inode, ranges,
offsets and supported permissions are parsed under fixed bounds. The
[proc maps manual](https://man7.org/linux/man-pages/man5/proc_pid_maps.5.html)
defines those metadata fields; they are not the mapped memory's authenticated bytes.

The existing helper and its 34 instructions remain unchanged. Original stack
header/program addresses are captured once and checked against all 272 bytes at
filter entry and zero exit, then non-dumpable hardening and existing native exec
denial complete in this same child. The useful change is evidence continuity:
the root proof, original invocation and installation no longer refer to different
helpers. This still does not create a production acceptance token or credential
channel. No arbitrary root, PID, command or acceptance bit is accepted by the owner.

Three eleven-case repeats pass, including real SIGKILL at initial, rooted,
first-exec and installed boundaries, sticky refusals, phase/replay, installer
faults and namespace/environment/tree/hash drift. Each repeat has eight seeded
fresh execs and four independent owners. Parser mutations, 75 previous regressions,
MSRV, portability, strict lint, normal-library exclusion and 142 root contracts
also pass. Development failures are retained: missing owner, missing Write import
and strict Clippy's nested-if rejection. They are not counted as successful fault
probes. The [design](../testing/performance/0.74/diagnostic-signer-rooted-exec-design.md)
and [source-bound packet](../testing/performance/0.74/local-runs/diagnostic-signer-rooted-exec-20261015/README.md)
separate these failed steps from final repeated results.

The important negative finding is that the unchanged policy still allows later
root/namespace mutations. Rejecting those operations during traced startup is
not equivalent to filtering them after installation. That is the next isolated
hypothesis, before dummy handoff or actual tracer-loss testing. The rooted fd
probe also stays deferred: its unchanged helper requires /proc/self/exe, absent
from the mount-free root; no convenient proc mount or helper rewrite hides this
gap. Independent runtime/issuer/job custody, memory attestation, authenticated
worker start, all writers and later lifecycle cells remain open. No allocation,
throughput or latency improvement was measured in this apparatus-only slice.

Publication exposed a separate reproducibility seam: a clean working tree was
not enough. Git normalized twelve Windows log files, so committed raw bytes no
longer matched their manifest hashes even though local verification passed.
The negative committed-object check is retained. Packet-local no-text attributes
preserve the actual captures, and an additional verifier compares every packet
file against its exact published Git object. Attribute/committed-byte substitutions
are now tested too. Evidence transport must preserve observations, not silently
rewrite them or compensate by changing expected hashes to normalized output.

## Restricting namespace root and mount changes after installation (2026-10-15)

The next local hypothesis was explicit confinement of the selected root, not
another throughput optimization. The old filter prohibited exec and task/control
operations but allowed namespace/root/mount syscall numbers. Rejecting them only
during traced startup did not constrain the post-install phase. The candidate
therefore preserves the old policy's ABI prefix and hardening tail, adding fifteen
EPERM pairs for namespace/root/cwd and legacy/new mount APIs. Its 64 instructions
are independently checked against all 512 bytes at original installation entry
and zero exit. This remains a private fixed helper, not a production sandbox.

Hardening exposed an observation seam. After dumpable-zero, fresh environ and fd
directory opens refuse EACCES. The first diagnostic omitted directory enumeration
and initially attributed the problem only to environ; the next failed run showed
that conclusion was incomplete. The owner now retains both original readonly
sources at the exact exec stop. Positional EOF reads check the original environment
mm, and bounded getdents64 obtains a live FD inventory rather than replaying a
saved list. Original namespace/root/cwd/executable/credentials/task/mapping checks
still bracket probes. Hardening was not disabled and inaccessible checks were not
silently omitted. Retained source identity remains dependent on original exec
history and exec denial, not mapped-memory attestation.

The planned regain counterexample also failed usefully. Under the old filter,
unshare(CLONE_NEWUSER) returned EPERM inside this chroot instead of creating
another namespace. That is consistent with the documented
[unshare chroot restriction](https://man7.org/linux/man-pages/man2/unshare.2.html).
No successful escape or regained capability is claimed. The exact failed result
is retained, and that control is refused/reaped before its helper continues.
Changing to unshare(0) or removing the root would test another hypothesis.
Three other old-policy controls distinguish successful chdir, ENOTDIR fchdir and
EBADF setns from candidate EPERM. Exact BPF semantics, not EPERM alone, attributes
denial where ordinary kernel permissions could produce the same result.

Three thirteen-case repeats pass, including all fifteen native denials, actual
rooted x32/i386 SIGSYS and original-child SIGKILL/drop at four boundaries. MSRV,
86 previous regressions, strict lint and 143 root contracts pass. The
[design](../testing/performance/0.74/diagnostic-signer-namespace-design.md) and
[source-bound packet](../testing/performance/0.74/local-runs/diagnostic-signer-namespace-20261015/README.md)
retain the compiler ICE, failed observation plumbing, rejected regain prediction
and lint correction alongside final results. This improves the test launch boundary;
it measures no cache goodput, allocations or latency. Bounded dummy handoff and
actual tracer loss come next, with production custody, authenticated worker start,
all writers and later lifecycle/server admission still separate.

## One shot synthetic transfer after original launch (2026-10-15)

The next hypothesis was whether launch proof could gate an actual bounded local
handoff rather than merely observe a process. A new fixed GNU helper uses the
existing original rooted owner and unchanged namespace policy, with one private
unnamed packet channel at FD3. No caller PID, descriptor or readiness assertion
can construct that owner. This evaluates bootstrap mechanics, not cache speed or
production credential custody.

READY is necessary but insufficient. The parent first verifies original exec,
exact filter installation, non-dumpable hardening, original context and the
16-byte READY send. The next original syscall must be the exact bounded read.
Only then does the parent latch its sole send attempt and transfer the public
dummy. It verifies the received stack bytes, ACK, channel close, final descriptor
inventory and zero original exit. A duplicate READY is rejected before transfer;
a lost or duplicate ACK is terminal after transfer and cannot authorize resend.
Unexpected FD-replacement and ancillary-capable syscalls are stopped at entry.
The inherited channel's creation credentials are not mistaken for the child's
PID; original trace history establishes the participant under this fixture's
trusted-launcher assumptions, consistent with
[UNIX socket semantics](https://man7.org/linux/man-pages/man7/unix.7.html).

The first startup guard was too broad: blocking all reads also blocked the GNU
loader before installation. The failed 2pass/9fail run and focused install error
remain retained. Restricting that guard to FD3 preserved ordinary selected-library
reads and all previous runtime checks. This was an implementation correction,
not a new filter policy, weakened bound or changed GNU profile. A mistaken test
target and a zero-test correction also remain visible; only the final correct
18+12 module runs complete that part of the regression ledger.

Three thirteen-case repeats pass, including pre-read withholding, malformed and
duplicate records, transport/ACK failures, context/endpoint faults, actual child
SIGKILL/drop at three transfer boundaries, seeded cases and independent owners.
All 99 old regressions, MSRV1.88, Windows portable9, strict lint and 144 source
contracts pass. The [design](../testing/performance/0.74/diagnostic-signer-handoff-design.md)
and [source-bound packet](../testing/performance/0.74/local-runs/diagnostic-signer-handoff-20261015/README.md)
separate these confirmed results from actual tracer-loss EXITKILL, which remains
the next experiment. A one-shot public dummy is not a signing credential, and
these tests do not authenticate production issuer/job custody, worker original-
start or cessation of all writers. Server and release admission remain closed;
no goodput, allocation or latency improvement follows from this safety fixture.

## Actual tracer death and independent child cleanup (2026-10-15)

Requesting EXITKILL is not evidence that owner loss is safe. The next local
fixture therefore kills the actual original tracer through its original pidfd
and observes the child from a separate process. At initial, rooted, original-exec,
installed, READY and delivered stops, the observer requires the original child's
CLD_KILLED/SIGKILL record, consumes it through P_PIDFD waitid and requires ECHILD
on repeat. It never signals the child on a successful path. A separate emergency
guard can clean a failed owned process group, but that cannot create a green
EXITKILL result. This remains a public-dummy experiment, not a production signer.

The observer itself needs an ownership boundary. It creates the original tracer
and private socketpair, receives kernel credentials and only five original FDs,
then checks the child pidfd, actual parent/tracer thread and exact root directory
inodes. Only the isolated observer becomes a subreaper; ordinary test-process
flags are checked unchanged. Root cleanup permissions change only after exact
original child death and reap. This composes the existing owner and unchanged
GNU helper rather than adopting a supplied PID or writing another signer engine.
The relevant kernel mechanisms are documented by
[ptrace](https://man7.org/linux/man-pages/man2/ptrace.2.html),
[subreapers](https://man7.org/linux/man-pages/man2/PR_SET_CHILD_SUBREAPER.2const.html)
and [pidfd waitid](https://man7.org/linux/man-pages/man2/waitid.2.html).

Several failed assumptions improved the experiment. The selected tree was nested
under `.tmpName/root`, not directly under the observer parent. The temporary
container's actual 0755 mode also contradicted an assumed 0700 default. The observer
now explicitly creates its own retained 0700 parent around that unchanged tree,
with descriptor-relative NOFOLLOW opens. The first coarse negative assertions
passed under those unrelated setup failures; final report faults are attributed
to their intended parser, frame, process or directory predicate instead.

The no-EXITKILL control also falsified its exact natural-exit prediction: installed
returned 74 rather than 73. Tracer death, ptrace detach and endpoint teardown are
not one atomic user-visible boundary. The unchanged helper can successfully send
READY before losing its peer, then fail to read a dummy that was never sent.
The failed prediction remains retained. A separately preregistered finite oracle
distinguishes only reachable CLD_EXITED branches from positive SIGKILL; actual
statuses are logged, not replaced by one convenient sample. Natural exit is not
evidence that a live orphan was reproduced.

Parallel current/MSRV runs exposed two harness confounds. An aggregate report
timer included compilation and every setup stage; bounded credential-checked
progress now separates those stages without increasing the five-second gap or
30-second outer bound. Progress is a hint and cannot admit the fault. A pipe EOF
test then failed while unrelated tests forked: CLOEXEC does not prevent temporary
inheritance of the same open description, as the
[fork semantics](https://man7.org/linux/man-pages/man2/fork.2.html) explain. Running
that oracle after exec in its own fixed role removes that confound without sleeps
or weaker assertions. Immediate EOF still proves received writers were closed;
the healthy report control keeps them alive until drop.

The [design](../testing/performance/0.74/diagnostic-signer-tracer-loss-design.md)
and [source-bound packet](../testing/performance/0.74/local-runs/diagnostic-signer-tracer-loss-20261015/README.md)
retain those failures beside three eleven-case repeats, 112 old regressions,
MSRV and 145 source contracts. The constructor window before installing EXITKILL
remains outside this proof. Independent runtime/job/issuer custody, real token
producer and authenticated receiver/replay/fence, worker original-start and all
writers, completion/cells 2–4 and server admission remain separate. No cache
throughput, allocation, latency or release qualification claim follows.

## Parent loss before EXITKILL installation

The previous tracer-loss proof started after the original owner had installed
EXITKILL. A real fork can outlive its parent before that installation. The next
local hypothesis therefore places PDEATHSIG(SIGKILL) at the beginning of the
same raw child and checks its original parent immediately afterwards. The
signal alone is not retroactive: if the whole parent already died, getppid must
detect reparenting and exit before descriptor, root or credential setup. The
kernel's [PDEATHSIG semantics](https://man7.org/linux/man-pages/man2/PR_SET_PDEATHSIG.2const.html)
make that second check essential. The guard remains active until the original
owner installs EXITKILL and resumes the child; only then may it clear.

The independent observer kills the original tracer at five constructor cuts.
Before arming it requires the exact parent-mismatch exit 81. At the four armed
cuts it requires SIGKILL, consumes the same original child pidfd's terminal
record and requires ECHILD before releasing the original runtime directories.
No child signal, replacement process, dummy input or emergency cleanup can
create a positive. Five healthy controls still complete the existing original
ACK/close/zero-exit protocol. Two negative controls omit only arming or only the
parent check and terminate with distinct natural exits 83/84 before exec. Those
outcomes cannot satisfy the positive oracle and are not live-orphan claims.

The first MSRV pressure run passed six tests and stalled in a healthy control
after root checks. At that point one five-second wait covered the entire
remaining constructor/root/exec/install/handshake chain. A separately committed
amendment adds seven credential-checked success hints with exact ordinals and no
FDs. Every gap retains five seconds; the outer bound remains 30 seconds and final
exit/reap/cleanup checks are unchanged. The final MSRV passes. The aggregate
wait structure is known; attributing the earlier stall entirely to pressure
remains a hypothesis rather than a proven kernel or product defect.

Verification selection also needs an oracle. A shell expansion removed a test
filter; other mistyped filters returned zero tests or the wrong scope despite
successful cargo exits. Those captures remain retained and are not promoted.
The final packet requires the exact per-suite vector, so deleting a suite or
changing 18 passes to zero fails even if the raw-log hash is updated. Three
seven-case repeats, all 123 prior signer regressions, MSRV, 146 source contracts
and Windows portability form the focused gate, not a release qualification.

The [design](../testing/performance/0.74/diagnostic-signer-constructor-loss-design.md)
and [evidence](../testing/performance/0.74/local-runs/diagnostic-signer-constructor-loss-20261015/README.md)
limit the result to whole-parent-process loss. Linux associates PDEATHSIG with
the creating thread, while getppid identifies its thread group. A thread dying
before arming while other parent threads stay live is a separate gap requiring
an explicit launcher thread model. Actual issuer/job/runtime custody, token
integration, worker original-start and all-writer cessation, later cells and
server admission remain separate. This is a lifecycle safety improvement, not
a measured cache throughput or allocation win.

## Creating thread death while the parent process remains alive

The whole-parent-loss guard did not settle one narrower race: getppid can still
match after the creating thread dies while other parent threads survive. The
next local experiment retains the actual creator's procfs directory before
fork and checks fresh status relative to it after PDEATHSIG arming. Pid, Tgid
and a live task state must match. It does not reopen a numeric TID, which could
refer to a replacement. The [kernel procfs documentation](https://docs.kernel.org/filesystems/proc.html)
explains why an open original object does not become a later task with the
same numeric identity. This is object binding, not mapped-memory attestation.

The fault must also isolate that thread. A whole-group SIGKILL would repeat the
previous experiment. Here the original callback invokes raw SYS_exit, leaving
its keeper and libtest/main threads alive. The
[exit syscall](https://man7.org/linux/man-pages/man2/exit.2.html) and
[PDEATHSIG semantics](https://man7.org/linux/man-pages/man2/PR_SET_PDEATHSIG.2const.html)
distinguish that lifetime from whole-process termination. All Rust preparation
finishes before the raw exit; the dedicated tracer process later exits as a
whole, without pretending its lost Rust thread resumed normally.

Before arming, the observer confirms the original creator is gone while the
parent process remains live, then releases the child gate. The new task check
refuses with exit81. The old-getppid control passes that earlier predicate and
exits86 before exec when only the new check is omitted. This reproduces the
missing condition without claiming a live authority-bearing orphan. At the
four armed cuts actual creator loss produces child SIGKILL.

The independent proof is not just the keeper's report. The observer requires
child pidfd readiness while the original parent pidfd is still nonterminal.
The keeper retains the child's WNOWAIT terminal record without reaping; only
then may the parent process exit. The independent subreaper subsequently
requires that same original child's kernel tuple, consumes it and requires
ECHILD before releasing runtime directories. Neither a direct child signal nor
emergency cleanup can make a positive. Healthy controls still complete the old
ACK/close/zero-exit chain and join their keeper.

Three eight-case repeats retain 51 positive terminal records, three separate
old-TGID controls and 15 healthy completions. All 130 previous signer regressions,
MSRV, 147 source contracts and Windows portable checks pass. A separate post-exec
role proves original task death, descriptor release and raw-reader FD stability;
the parser rejects dead/future/duplicate/oversized status under fixed stack/read
bounds. No duration or qualification threshold changes. The
[design](../testing/performance/0.74/diagnostic-signer-creator-loss-design.md) and
[evidence](../testing/performance/0.74/local-runs/diagnostic-signer-creator-loss-20261015/README.md)
separate this confirmed constructor result from still-unproved post-options
tracer-thread-only loss, forced TID reuse and arbitrary signer-thread support.
Actual launcher/runtime/job custody, token integration, worker original-start,
all-writer cessation and server admission remain separate. No measured cache
performance improvement follows from these lifecycle tests.

## EXITKILL when only the original tracing thread exits

Whole-process loss and pre-options creator loss leave a distinct question:
does the child die when its original tracing thread exits, although another
thread keeps that same process alive? The new local fixture uses the unchanged
original rooted exec and dummy handoff, then invokes raw SYS_exit in the actual
creator/tracer task. A keeper deliberately preserves its original TGID. Linux
documents [per-thread tracing and EXITKILL](https://man7.org/linux/man-pages/man2/ptrace.2.html);
the test checks that lifetime directly instead of substituting a group kill.

Six cuts cover initial options, rooted state, first exec, installed policy,
READY and delivered dummy. The initial cut still has both PDEATHSIG and EXITKILL;
it cannot attribute the kill exclusively to EXITKILL. The existing child clears
PDEATHSIG on first resume, before the other cuts. At those later cuts, the exact
original child's SIGKILL while its original TGID remains nonterminal provides
the narrower EXITKILL result. The observer retains original task and child
objects before release, verifies the kernel parent/tracer relationship and root
objects, then independently corroborates and reaps the keeper's WNOWAIT tuple.
No positive path signals the child itself.

The falsifying rooted control removes only EXITKILL. After original thread exit
the child is detached and nonterminal while its original TGID remains alive.
Only after recording that bounded point observation does the observer signal
the validated original child pidfd for cleanup. Its later SIGKILL has
`positive=false`. This separates kernel protection from the test's cleanup;
calling both terminal records successes would erase the experiment's meaning.
Healthy controls still complete the original ACK, close and zero exit at all
six cuts, with the keeper joined normally.

The first repeated run also exposed an observer mistake: it waited for several
successful advances as if they were one five-second protocol gap. A healthy
late-cut case timed out. The failed log remains retained. A separately recorded
amendment adds exact credential-checked progress ordinals between the original
steps; five-second gap, thirty-second outer and capture bounds stay unchanged.
Three final six-test repetitions retain 54 positive deaths, three negative
controls and 18 healthy completions. All 138 prior signer regressions, the six
MSRV tests, 148 source contracts and nine Windows portable tests pass. These
counts describe correctness checks, not cache throughput or allocation gains.

The [design](../testing/performance/0.74/diagnostic-signer-thread-exit-design.md)
and [evidence packet](../testing/performance/0.74/local-runs/diagnostic-signer-thread-exit-20261015/README.md)
close this fixed thread-loss gap while keeping real runtime/job/issuer custody,
actual credential integration, authenticated worker start and cessation of all
P2/P3 writers separate. Those are still needed before completion cells 2–4 and
server admission; no key, host service or qualification input changed here.

## Preparing a signer issuer without enrolling production authority

The local launch and thread-loss proofs do not choose who may certify the
actual signer build. That needs a separate issuer from the controller, observer
builder and worker-policy roles. A new local Ed25519 pair now has a role-fixed
generation/inspection helper, different filenames and a non-receipt selfcheck
domain. Public role and canonical shape are checked before a private file is
opened; old reviewed key roles, unknown fields, changed admission flags, links,
permission drift and replacement refuse. Existing or partial state cannot be
overwritten or repaired by rerunning generation.

The real pair was generated once under an owned 0700 WSL directory, with a 0600
single-link unencrypted private file and a 0400 public record. Only public data
entered the repository. A second inspection matched exactly, and an independent
Rust verifier checked the public fingerprint and strict Ed25519 selfcheck on
Windows and Linux. This proves a local key/selfcheck relationship, not approved
production ownership, storage encryption or an authorized build. The record
keeps pin approval and production enrollment false until the operator selects
the fingerprint explicitly. The [operator note](../testing/performance/0.74/diagnostic-signer-issuer-key.md)
documents storage, commands, future signing domain and rotation without exposing
the private material.

The other preparation is an explicitly disabled signing-job draft outside
GitHub's workflow directory. Both jobs fail unconditionally even if their false
guards are removed. Its future compilation/dependency phase has no signing or
OIDC authority; its future signer phase must consume independently admitted
prebuilt launcher, signer and GNU runtime instead of compiling in that lifetime.
GitHub's [OIDC permission is job-scoped](https://docs.github.com/en/actions/reference/security/oidc#workflow-permissions-for-the-requesting-the-oidc-token),
so clearing one process environment cannot remove authority from an already
untrusted job. Exact workflow/subject/protection, runtime/root custody and issuer
snapshot validity remain unresolved, rather than filled with plausible defaults.

Three 23-case Python runs, three strict public-verifier tests on both parent
toolchains, existing signer/trust regressions and 149 source contracts pass.
Windows explicitly skips Linux-only custody cases. The
[evidence packet](../testing/performance/0.74/local-runs/diagnostic-signer-custody-20261015/README.md)
distinguishes disposable crypto fixtures, actual local generation and the
unapproved production boundary. No build receipt, worker policy or OIDC token
was produced; no existing key, GitHub configuration, server or qualification
input changed. Authenticated prebuilt custody and token coordination come next,
then worker original-start and all-writer cessation before completion cells 2–4.

A [decoder followup](../testing/performance/0.74/local-runs/diagnostic-signer-custody-depth-20261015/README.md)
adds explicit sanitization of a recursion exception before private access. The
initial deep-input prediction did not reproduce a failure on Windows; that
green observation is retained separately from a deterministic injected-exception
test. The final helper has 24 cases and passes three Linux repeats, with 14
explicit Linux-only skips on Windows. The earlier generation packet remains
immutable and source-specific. No key was regenerated or fingerprint approved.

## Selecting the issuer and inspecting the actual unsigned artifact

The operator subsequently selected the public fingerprint for the separate
signer-build issuer role. That decision now has a code pin with no caller key,
environment or input-record override. It does not rewrite the earlier generation
record or its selfcheck. The new factory binds this key to an explicitly asserted
expected build statement and returns the same diagnostic-only model, without a
production loader or bootstrap capability. A synthetic receipt that passes the
old caller-key model is rejected by the selected-key factory with identical
statement and content. Three six-case repeats, compile-fail override coverage,
old signer/trust regressions, current/MSRV checks and 150 contracts pass.

The next observation uses the actual diagnostic_controller rather than the
small launch helper: one clean-source local Rust 1.94 release/System/GNU build,
with no signing credentials or product work. It produces a 1,922,976-byte unsigned
PIE executable. Readelf records the expected GNU interpreter and three direct
library declarations, with no RPATH/RUNPATH and version needs including GLIBC_2.34.
The [readelf manual](https://sourceware.org/binutils/docs/binutils/readelf.html)
describes these static ELF views; they do not establish which files a live loader
will use, later loads, TLS inputs, protected ancestry or absence of writers.
The binary was not executed, signed or admitted as a product candidate.

The build also exposes a concrete custody gap: Cargo's output is 0755 with two
hardlinks, while the existing original bundle-reader requires 0555 and one link.
The observation is retained without chmod, unlink or a copy being promoted to
trusted custody. A stable digest around inspection cannot revoke retained
writable descriptors or authenticate a runtime root. Safe original prebuilt
preparation and independent runtime/job selection remain the next implementation,
followed by real retained-body credential coordination. Worker original-start,
P2/P3 all-writer cessation and completion cells 2–4 remain separate prerequisites.
The [new packet](../testing/performance/0.74/local-runs/diagnostic-signer-pin-20261015/README.md)
separates confirmed key selection and observed unsigned build facts from those
unimplemented joins; no throughput or allocation improvement is claimed here.

## Preparing the unsigned copy and killing the preparer

Cargo's writable two-link output is now copied into an exclusively created
attempt directory. The Linux preparer accepts original read-only descriptors
and an asserted size/hash, copies positionally with a fixed buffer, syncs the
file, changes only the new inode to 0555, checks the explicit writer close,
reopens read-only and syncs the directory and parent. It returns inventory, not
a file descriptor or a conversion to a trusted bundle. Existing and interrupted
attempts cannot be overwritten, repaired or resumed.

Repeating the tests exposed a real gap in the first implementation: the
metadata-only bracket sometimes accepted a source write immediately after
copying. The copied bytes still matched the expected digest, but the claim
that the source had not changed was unsupported. The fix adds a second content
pass over the original source, bracketed by metadata checks. It does not sleep
until timestamps differ or weaken the test. The failed precommit observation
is retained separately from the final passing source; it is not a clean-source
benchmark candidate or a continuous source-immutability proof.

The owned-process fault test pauses a libtest subprocess at seven copy
checkpoints and SIGKILLs/reaps that exact child. All seven retained attempts
refuse reuse. The final cut deliberately leaves complete-looking bytes and
readonly modes: apparent completion is not permission to repeat execution.
Three current-toolchain repeats and MSRV checks pass alongside copy integration,
old bundle/issuer regressions, compile-fail API guards and 151 contracts. This
tests process loss, not power failure or the worker's authenticated lifecycle.

One actual local preparation of the previously inventoried unsigned controller
produced a distinct inode, 0555 and one link, with unchanged size and SHA256.
The Cargo source stayed 0755 with two links. Neither controller file was run or
signed. The [copy packet](../testing/performance/0.74/local-runs/diagnostic-prebuilt-copy-20261015/README.md)
records source and preparer identities separately.

The foreign-writer counterexample remains decisive. A descriptor opened during
creation can write after chmod even when the first inventory check passes.
Later revalidation detects the write and refuses permanently, but detection is
not revocation. Production still needs protected ancestry/account/runtime/job
custody and independently authenticated receipt inputs before signing. Original
worker start and cessation of every P2/P3 writer still precede completion and
cells 2–4. These changes establish copy mechanics and failure boundaries, not a
cache throughput or allocation improvement.

## Retaining the root owned path in the original reader context

The copy preparer answered which bytes reached a new inode and when its own
writer closed. It did not authenticate the directories that name a runtime
input. The next reader retains every path component from the current `/`, fixes
numeric UID/GID 0 without a caller override, and rejects writable peer modes,
symlinks, special files and hardlinked leaves. It checks required statx fields,
including mount ID, and refuses missing support rather than falling back to
device/inode alone. Ordinary mount IDs remain observations, not globally unique
or cross-reboot authority.

Reading context is retained with the path: the original reading TID, current
root and typed nsfs user/mount namespace objects. Fresh name walks and positional
hashes are bracketed by context checks; the first failure permanently refuses
the guard. The object cannot move across threads, export an FD, deserialize,
clone or become a bundle capability. Its private owner/anchor seam exists for
owned filesystem tests, not as a public alternative to the numeric-root entry.

Leaf metadata remains exact, but ancestor timestamp changes caused by unrelated
entries do not reject an otherwise unchanged path binding. Tests distinguish
that case from file/ancestor replacement and owner/mode/mount drift. A content
test deliberately refreshes private fixture metadata after a same-size write;
the hash must still refuse. All 13 leaf stamp fields have explicit mutation
coverage within the seeded 64-case checks, with independent concurrent observers.
Three 12-case current-toolchain repeats, MSRV, two integration cases, six
compile-fail API cases, old copy/bundle/issuer regressions and 152 contracts pass.

One clean-source local inventory then reads libc, libgcc_s and the GNU loader
without requesting a separate execution or copy of those inputs. Ordinary
inventory/stat/hash tools still use the host runtime; their startup and loaded
closure are not authenticated by this reader. It accepts all three
explicit paths, and their hashes/metadata match before and after. libgcc_s is
0644; libc and the loader are 0755. These ordinary root-owned system modes are
allowed only by this inventory profile, not substituted for the old bundle's
0555 contract. The previously prepared 0555, single-link controller copy still
belongs to UID/GID 1000 and is refused through the public root-owned reader.

The [packet](../testing/performance/0.74/local-runs/diagnostic-root-owned-input-20261015/README.md)
separates those observations from custody. Numeric root in a caller's namespace
does not select the initial host or authenticate its filesystem/administrator;
matching file hashes do not select a job, prove the loaded closure or revoke a
foreign writer. The local example drops its live guard at exit, so the retained
packet is not a live capability. Independent host/runtime/job policy and receipt
acquisition remain before signing, original worker start, P2/P3 cessation and
completion. No cache performance improvement or server admission follows.

## Inspecting the actual enrollment boundary

The next question was whether the local trust primitives could simply be filled
with facts from the existing dedicated server. An operator-authorized read-only
inventory obtained account IDs, account-file hashes, machine/boot and namespace
observations, installed receipt/binary identity, fixed service properties and
GitHub policy. It did not install trust or run a candidate. The stable supervisor
remained active with the same PID and zero restarts; the lifecycle snapshots
showed no active campaign or fixture marker. Those snapshots do not reserve the
host against another start.

The useful falsification was a namespace mismatch. The SSH reader and PID 1
shared mount namespace inode 4026531841, while the confined supervisor occupied
4026532718. A host inventory is therefore not automatically the receiver context
that the kernel-policy join will compare. Copying the SSH namespace into a signed
policy would authenticate the wrong claim. Disabling confinement to make the
numbers match would discard a protection instead of fixing the enrollment.
The receiver must be enrolled from its actual controlled lifetime after an exact
installation; replacing it cannot silently refresh its original namespace pins.

A second misleading shortcut appeared in cgroup inspection. Root lacked
`cgroup.type` and `cgroup.kill`, but the real supervisor cgroup reported `domain`
and exposed `cgroup.kill`. An initial collector that required the root-only
probe aborted; its failure and correction are retained. Neither the corrected
file presence nor a stable service name proves permission to revoke an original
worker cgroup or cessation of every pipe/file writer. No kill operation was
performed to turn inventory into a lifecycle proof.

GitHub configuration supplied another concrete boundary. The dedicated diagnostic
controller workflow and environment did not exist. The old reference environment
had signing-secret metadata but no environment protection rules; its existence
could not supply the future protected signing job. The public OIDC snapshot
reproduced four keys, while the actual default-subject settings allowed a specific
environment subject to be proposed. There was still no received job token,
selected workflow revision or independently admitted prebuilt signer/runtime.

The resulting [enrollment review](../testing/performance/0.74/diagnostic-enrollment-review-20261015.md)
keeps observations and decisions separate. It proposes the real worker account,
a deliberately explicit new-authority initialization and a bounded four-key
roster, without claiming to discover an epoch from an absent directory. Receiver
namespace and signed-body digest remain unresolved. Approval of the model must
precede actual configuration, then original-start, watchdog/all-writer cessation,
completion and cells 2–4. These findings reduce deployment uncertainty; they do
not measure cache allocation, throughput or readiness to ship.

## Sharing declaration checks before admitting the signer runtime

The enrollment review exposed a code-level gap as well as missing deployment
facts: GNU declarations were checked only by test fixtures, while signer-build
runtime/context digests still treated their bodies as opaque bytes. The next
local slice moved the existing strict ELF parser and four-role graph into the
normal library. Owned fixtures import that same implementation, so their parser
mutation and rooted-exec regressions exercise the code used by the ordinary
catalog entry rather than a second test-only validator. Bounds, unknown-tag
refusal and GNU role rules were preserved.

The catalog accepts exactly signer, loader, libc and libgcc byte snapshots and
records their lengths, hashes, interpreter, NEEDED, SONAME and header shape.
Portable tests construct independent synthetic images, change role/ABI/range/
search-path/size inputs, run 256 seeded no-panic mutations and compare four
independent readers. A byte outside the declaration tables can change without
changing those declarations; the recorded digest still changes. This distinction
matters because a syntactically accepted ELF is not necessarily safe to load.

One clean implementation commit then supplied an ordinary local inspection
example. It read the existing unsigned 1,922,976-byte controller copy and three
GNU files, without requesting execution of the controller or ldd. Their declared
graph passed; swapped loader/libc roles refused. Independent hashes and metadata
matched before and after. Ordinary inspection/compiler tools still use their
host runtime, whose actual mappings are not certified by this catalog.

Current and MSRV tests, old runtime/rooted-exec regressions and the new source
contract pass. A development source guard initially depended on rustfmt's line
layout; preserving its ordering check over whitespace-normalized source fixed
that test without changing a runtime limit. The negative is retained with the
missing-API baseline and the other local evidence.

The [catalog packet](../testing/performance/0.74/local-runs/diagnostic-signer-gnu-catalog-20261015/README.md)
is a preparation result, not a signed runtime admission. Its custody, loaded
closure, original-start, signing and admission flags remain false. The next join
must bind independently selected original prebuilt/runtime objects and their
protection before receipt signing or token exposure. Receiver enrollment,
all-writer cessation and completion cells 2–4 remain separate. No cache
performance claim follows from making a tooling validator reusable.

## Binding receipt contents to original runtime files

After extracting the GNU catalog, a signed runtime-manifest hash could still
refer to arbitrary supplied bytes. The next hypothesis was narrower than runtime
admission: require that this receipt slot describe exactly the files retained
by the original reader. The new Linux join obtains signer bytes from its retained
signer descriptor, generates the four-role catalog from those same original
inputs and checks canonical sorted-key JSON plus one LF as runtime-manifest
content. The caller can supply lock, log and bootstrap context, but cannot provide
an alternate binary or runtime manifest through that entry.

The signature authenticates the exact caller-selected expected statement before
file acquisition. The old numeric-root reader checks ancestors, original context,
metadata and content around bounded positional reads; its public owner/anchor
policy is unchanged. Four readers remain retained for subsequent checks. An
equal-byte replacement is still a different original object, and any error
permanently refuses the complete owner rather than accepting restored names.

Synthetic tests falsify the tempting shortcut that a valid signature should be
enough: correctly re-signed opaque manifests, alternate encodings and catalogs
of changed dependencies all refuse. Tests also cover each companion and role,
wrong key/domain/signature, equal-byte replacement of every role, content and
ancestor drift, 64 seeded signed substitutions and four thread-local owners.
The old build, GNU, issuer, bundle and runtime suites remain green. An initially
unregistered Linux-only test target failed the gate; registering it in the
existing fast tier preserves coverage without changing a workflow or threshold.

The [runtime join packet](../testing/performance/0.74/local-runs/diagnostic-signer-runtime-join-20261015/README.md)
records consistency under synthetic selected keys, not an actual production
receipt or an independently trusted filesystem. Numeric root ownership and
sequential rehashing cannot revoke retained writers or establish launcher/job
custody. Image buffers are bounded and dropped after construction; no cache
allocation or throughput improvement was measured. The next production join
still requires independently admitted runtime/root/launcher and protected job
inputs, followed by original-start and cessation of every P2/P3 writer.

## Checking the bootstrap context inside a signed receipt

The runtime join removed an opaque runtime-manifest seam, but the receipt could
still bind any bootstrap-context bytes. The next isolated change adds a stronger
optional reader for that inner document. It checks exact signer source/tree,
binary and runtime-manifest bindings, a separate bounded launcher assertion and
the fixed controller workflow/environment/default-subject profile. An explicit
nonzero workflow revision is required; no actual revision is chosen by the parser.

The declared build must have neither signing nor OIDC authority. The declared
signing job must not compile, install dependencies, checkout code or inherit an
unreviewed environment. Correctly re-signing a context with any of those permissions
enabled passes the old opaque receipt model but fails the stronger path. Strict
canonical LF encoding and schema/type/duplicate checks prevent ambiguity. The
Linux composition checks this context before acquiring its original GNU files,
then retains the existing sticky original-file checks. The old APIs remain
compatible and no installed job or producer is changed.

Tests cover every field, content/signature drift, 64 signed schema mutations and
four independent models. Existing OIDC regressions pass after sharing the fixed
workflow/environment strings, and the full root suite now passes 155 contracts.
The [context packet](../testing/performance/0.74/local-runs/diagnostic-signer-bootstrap-context-20261015/README.md)
also keeps an important negative control: explicitly reselecting another complete
expected statement with well-formed launcher/workflow pins can pass syntax.
Consistency cannot independently authenticate those pins or prove that a job
actually obeys signed requirements.

This closes malformed and internally inconsistent declared contexts, not the
remaining launcher/root/job custody or writer revocation. No real private key,
token, production receipt or server execution was involved, and no cache
performance measurement follows from this tooling change. Actual enforcement
and independently reviewed original-start still precede completion and cells 2–4.

## From declared protection to a fixed ordinary child entry

The next isolated change is actual kernel enforcement, but still not a complete
launcher. Previously, the 64-instruction namespace restriction policy and its
installer lived only in fixtures. The ordinary library now generates the same
512 bytes, with a private Linux installer reachable only through a terminating
fixed-probe entry. A normal binary accepts native/x32/i386, not a caller-selected
executable, PID, FD, filter, key or command. The old independent generator and
standalone helper remain unchanged regression oracles.

The fresh secret-free child refuses root numerical credentials, unequal UID/GID
slots, capabilities, multiple threads, extra FDs or a nonempty environment. It
disables core dumps, sets/verifies no_new_privs, requires exact zero from TSYNC,
sets ND0 and rechecks the current context and exactly one additional filter.
Failures terminate the child, including partial installation; no installed guard
or bootstrap/signing capability returns to the caller. These checks observe the
current namespace, not initial-host/account or independent job custody.

The useful experimental contrast is small and concrete: unfiltered chdir('/')
succeeds in an owned control child; the ordinary filtered probe receives exact
EPERM. Twenty fixed safe native calls are checked, and actual x32/i386 getpid
probes die SIGSYS with no output/core. Fork/clone numbers are covered in the
independent model and old fixtures, not invoked by this new probe. Null pointers
and invalid FDs avoid executing or mounting anything if the filter is defective.
The existing prctl tail checks both full-width arguments for ND0 but ignores
four unused arguments. Other syscalls remain allowed: a deny list is not a full
sandbox or mapped-instruction attestation.

Repeating the experiment mattered. An initial green run was followed by exit2
failures on both current Rust and MSRV. Temporary diagnostics found an inherited
FD7 alongside stdio and the probe's own proc-directory FD3. Its origin was not
authenticated. The restriction was working; the test launcher had failed to
establish its own clean child precondition. The harness now requires a child-only
CLOEXEC sweep before exec. The inherited-FD negative deliberately clears its own
flag afterward and still must refuse. No parent flag or normal installer guard
was relaxed. Temporary diagnostics are removed, while failed captures remain.

After that correction, eight Linux repeats pass five normal-process and seven
portable tests each, with parent seccomp/core/cwd unchanged across four independent
owners. Three decoder tests cover malformed/bounded status and FD inventories;
root and multithread negatives there are models, not privileged execution.
MSRV 1.88 and Windows portable checks pass, as do strict lint, 156 root contracts
and the old namespace/rooted-exec/handoff suites (13/11/13). The
[child-entry packet](../testing/performance/0.74/local-runs/diagnostic-signer-child-entry-20261015/README.md)
also preserves initial registry/lint failures and explicitly marks a truncated
compiler diagnostic as partial, not passing evidence.

What this justifies is reuse of one tested restriction primitive outside cfg(test).
It does not establish original executable/root provenance, protected runtime/job
custody, credential handoff or cessation of every P2/P3 writer. Stdio-only entry
cannot yet implement the fixture's FD3 handoff. The next work is an owned original
launcher with explicit root/runtime/descriptor and parent-loss lifecycle, followed
by authenticated original-start and all-writer fencing. Signing, completion,
later cells and server admission remain closed. No product workload or cache
allocation/throughput result was measured, and no server or GitHub state changed.

## Ordinary self exec ownership without signer admission

The ordinary probe now connects the fixed child restriction primitive to an
original process owner. A separate normal binary retains its own read-only
0555 image, forks exactly one child, acquires its pidfd and requires the first
kernel PTRACE_EVENT_EXEC. The stopped kernel executable must match the retained
original object before resume. Only then does the existing fixed child entry
run. No caller-selected executable, PID, root, FD or command is accepted, and
the owner remains private to the binary rather than a public launch capability.

The parent is a fresh nonroot main-thread process with empty environment. Before
fork, it also requires default SIGCHLD without autoreap: an ignored signal could
invalidate the assumption that the fork PID remains unreaped before pidfd
acquisition. This guard queries rather than rewrites the caller's disposition;
the actual inherited-ignore negative must refuse. Child setup arms and reads
back PDEATHSIG, checks the captured parent, closes foreign descriptors before
the initial barrier and requests TRACEEXEC plus EXITKILL before original exec.
Phase misuse or unexpected events permanently refuse, and cleanup targets only
that original child. Once reaped, the owner cannot revisit an old numeric PID.

The first passing test run was not sufficient. Parallel copy and outer test
startup later produced ETXTBSY. A retained writable descriptor is consistent
with this failure, even after changing a file to 0555; CLOEXEC does not close a
fork child's copied descriptor until it reaches exec. The actual writer in the
failed run was not identified, so this attribution remains an inference. The
harness now serializes only copying and startup through exec, not normal owner
lifetimes. A deterministic held-writer control requires exact ETXTBSY without
retry; only explicit writer drop permits a new independent probe. Permissions
alone therefore cannot stand in for the later all-writer revocation proof.

Five final Linux repeats each pass nine private unit tests, one portable CLI
test and five normal-process tests. Native/x32/i386 traverse the ordinary owner;
the existing child tests independently retain exact EPERM and SIGSYS checks.
Actual owned SIGKILL/drop is exercised at initial and first-exec stops. A
foreign equal-byte file, phase replay, malformed image and exit0 without the
expected fixed line refuse. Status classification covers 327680 cases plus
256 seeded mutations, with four independent owners and unchanged parent state.
MSRV 1.88, strict lint, 157 root contracts and the 52 prior first-exec/namespace/
handoff/constructor-loss/creator-loss regression tests pass. The
[owned-probe packet](../testing/performance/0.74/local-runs/diagnostic-signer-owned-probe-20261015/README.md)
retains failed current/MSRV runs and the lint correction as well as final repeats.

This establishes original self-exec mechanics for one secret-free diagnostic,
not independently authenticated signer provenance. Its GNU loader/runtime are
still current-environment inputs; an environment check after startup cannot
retroactively authenticate loader execution. PDEATHSIG is a precondition, not
an authentication token, and requesting EXITKILL is not a proof of every
parent-loss boundary in this new composition. The next joins remain selected
root/runtime and mapped-image checks, original-owner loss and protected job
custody before credential handoff. Worker original-start and all-writer fencing
are separate prerequisites to completion and cells 2–4. No product workload,
performance claim, server/GitHub mutation or qualification follows from these
local process tests.

## Original owner loss at two existing child stops

The next test kills the original owner process, not its child. A dedicated monitor
retains the owner and child pidfds and checks the original creating task, sole
child, PPid and TracerPid. At the initial stop and first kernel exec stop it sends
SIGKILL only to the owner. Both cases require the adopted original child to report
CLD_KILLED/SIGKILL, then exact reap and ECHILD. An unrelated original process stays
alive. Healthy controls grant a private acknowledgement and let the normal guard
clean its child. At the initial cut EXITKILL is not yet installed; at first exec
both protections exist, so this observation does not attribute the kill to one.

The first harness incorrectly treated self-SIGSTOP as an acknowledgement. Its
exec case observed a zombie before fault injection. A successful signal send is
not a logical instruction barrier: the owner could continue into its loss-path
assertion and cleanup. The replacement socket gate allows cleanup only after the
healthy grant. The failed captures remain in the
[owner-loss packet](../testing/performance/0.74/local-runs/diagnostic-signer-owned-loss-20261015/README.md).

Five Linux repeats pass 12 unit tests, one portable CLI test and five normal CLI
tests each; Rust1.88 passes the same groups. The unit total includes a no-role
entry helper. Strict lint, current/MSRV checks, Windows refusal portability and
158 root contracts pass. This loss matrix calls the unchanged private constructor
from test roles with a fixed /bin/true image. It is not a proof of every boundary
through the complete normal main-thread CLI, independently selected root/runtime,
credential handoff or P2/P3 all-writer cessation.

An expanded old creator/thread-exit regression group fails at its no-EXITKILL
negative control in both default parallel runs, while all14 pass serially. Those
library sources are unchanged; the cause is not yet attributed. Serial success
does not repair the default gate, so all-regressions-passed remains false and this
failure must be investigated before server admission. No performance result or
release readiness follows from the new owner-loss checks.

## Kernel group stop and joint observation of tracer loss

The parallel no-EXITKILL failure is now attributed to the fixture's stop and
observation boundaries. Its rooted SIGSTOP was a signal-delivery-stop, not a
job-control group-stop. After tracer-thread exit the detached child could run
the unchanged helper and terminate before observation. A failure captured a
zombie child with TracerPid0 and Seccomp2 while the original TGID remained alive.
The helper's nonblocking receive without an offered dummy is consistent with
that natural termination, but the exact exit code was not recorded.

Both rooted loss variants now establish the same kernel group-stop before
their EXITKILL options diverge. GETSIGINFO must identify the original SIGSTOP
delivery and then fail with exactly EINVAL after injection and the next stop.
The first correction alone was insufficient: a repeat saw TracerPid0/StateR
while both originals were live. Detach and job-stop must be awaited together.
The final monitor requires TracerPid0 plus StateT and live original pidfds;
R/t remain pending under the original deadline, never success. Death still
refuses. This follows the documented distinction between
[ptrace stops and group stops](https://man7.org/linux/man-pages/man2/ptrace.2.html)
without inferring an atomic transition from separate kernel fields.

Five final default-parallel repeats each pass17 tests; five additional runs
pass the four-independent-negative-observer control. The deterministic stop
and transition regressions were red before their respective corrections.
MSRV1.88 passes17 tests, unchanged launcher/child and old fixture regressions
pass, and158 root contracts pass. No deadline, terminal tuple, global scheduling
lock, fixed helper or normal launcher code changed. An extra pidfd SIGSTOP was
considered but not added. The
[race packet](../testing/performance/0.74/local-runs/diagnostic-signer-thread-exit-race-20261015/README.md)
retains both the original and intermediate failures; earlier packets remain
historical evidence, not silently rewritten successes.

This closes the observed local fixture race, not server readiness. Independently
selected root/runtime, full normal-entry loss and protected handoff remain before
authenticated worker start and all-writer fencing. No cache-performance claim,
credential access, signing, server change or qualification follows from this fix.

## A selected root must exist outside the test harness

The next step moves a small, concrete prerequisite into the normal build: preparing
the fixed GNU filesystem tree. The previous root and rooted-exec fixtures remain
independent regression oracles. The new binary-private preparer neither promotes
their test-only launcher nor accepts an arbitrary command. Its single `prepare`
mode creates, checks and removes a root; it does not execute a child inside it.

The hypothesis is about ownership and reproducibility, not cache throughput:
ordinary code should retain the exact source objects and construct a bounded,
verifiable four-role tree without depending on test-only TempDir cleanup. The
sources are self, the fixed GNU loader, libc and libgcc. Original read-only
CLOEXEC descriptors, metadata and byte hashes stay owned together. The existing
GNU declaration parser and 8 MiB/image bound remain unchanged. Fixed distribution
symlinks are deliberately followed during acquisition; numeric ownership checks
do not turn their ancestry or the current namespace into authenticated provenance.

One random 128-bit name is created under retained sticky `/tmp`; collisions refuse
without adoption or retry. Exclusive descriptor-relative creation builds exactly
four directories and four files. Temporary writable copy descriptors close before
return; finalized objects have mode0555. Rechecks validate the original descriptors,
root name bindings, bounded inventories, metadata and source/copy bytes. Failure
latches rather than replacing an original with a newly observed equivalent object.

Cleanup is part of the hypothesis. Before changing permissions or unlinking
anything, the owner checks every known binding and inventory. It then removes only
the four known files, three subdirectories and original fresh root name. Equal-byte
file replacement, a foreign root, directory symlink or extra entry refuses before
cleanup mutation. No recursive deletion is used. Tests cover eight owned partial
construction prefixes too. This is still a cooperative local scope: hostile same-UID
or privileged races between validation and unlink are not defeated, process SIGKILL
can leave a root behind, and mode0555 is not independent custody or writer revocation.

The compiler comparison provided a useful negative result. The local Rust1.94
private debug executable, with debug sections stripped only from the test copy,
fits the four-role declaration graph and prepares real runtime files. The local
Rust1.88 executable additionally needs libm. Initial process expectations failed;
readelf identified that dependency difference. Expanding the graph would change
the selected profile, so the parser was left intact. MSRV compilation and synthetic
unit materialization pass, while its independently identified extra-libm normal
profile deliberately refuses. That refusal is not MSRV runtime support, and neither
private debug copy is a production release-build attestation.

Another test incorrectly assumed that source-path renaming must change original
metadata. The retained source descriptor, not its former pathname, is the input
being checked. The corrected test keeps exact stamp/byte semantics and verifies
that a foreign non-ELF replacement is never adopted. Created-root path bindings
remain a separate mandatory check. This distinguishes retaining an original object
from claiming custody of every name once used to find it.

Final checks pass11 unit, one CLI and three process tests on Linux Rust1.94, including
four independent normal preparations. MSRV passes those counts with the explicit
runtime refusal above. Windows current/MSRV CLI refusal, strict checks/lint,
159 root contracts,175 registered gates and local documentation/contracts pass;
ship admission still refuses. The
[local packet](../testing/performance/0.74/local-runs/diagnostic-signer-owned-root-20261015/README.md)
retains initial failures, exact source/tree, captures and15 verifier tests.

The wider baseline also found an unrelated unresolved boundary: the unchanged
old-root group failed78/1 in its constructor-loss seeded observer. The same root
and constructor groups later passed7/7 each when focused. That is an observation,
not attribution or repair; all-regressions-passed remains false. The next priority
is to explain that failure before composing normal root selection with original
child launch and kernel post-exec mappings. Existing child restrictions depend on
procfs and must not be weakened to accommodate a proc-less root. Protected handoff,
full normal-entry loss, authenticated worker start and all-writer fencing remain
open. No server operation, performance sample or release-readiness claim follows.

## EOF is not original creator termination

The broader constructor failure above exposed a fixture ordering problem. Before
arming PDEATHSIG, the raw child waited for EOF from a private gate whose peer
belonged to its tracer. The fixture then treated that EOF as though the original
creator had already finished terminating. A fresh baseline and an instrumented
broad group each passed79, which explains why a successful rerun was insufficient.
The retained failed seeded loop used fault=None/healthy=false; its closed oracle
rejects SIGKILL only at before-arm, where it requires exit81. That identifies the
failed cut without inventing a missing historical syscall trace.

The controlled experiment separates the events. While the original creator is
alive, the fixture forces EOF through the same gate. The unchanged raw guard
passes its PDEATHSIG readback and parent checks and reaches the first ptrace stop.
The callback has not returned to PTRACE_SETOPTIONS, so EXITKILL is not installed.
Killing only the original creator then yields original-child SIGKILL, exact pidfd
reap and ECHILD. EOF clearly did not establish creator death before arming. This
demonstrates the ordering defect; it is not a syscall recording of the earlier
failed run or a positive before-arm exit81 result.

The correction belongs in the observer, not in the guard or terminal oracle. The
independent monitor retains the original gate peer through exact original tracer
termination and reap. For unarmed cuts it also requires the original child pidfd
to remain nonterminal before dropping the peer. Only then does EOF release the
child. Before-arm still requires exit81; negative controls still require83/84,
and armed cuts still require SIGKILL followed by original reap/ECHILD. The new
barrier regression was red before this change even when the old terminal result
happened to be81. Cleanup never substitutes for a passing death observation.

One bounded setup packet carries exactly `G` and the peer FD before original fork.
Socket kind/domain, CLOEXEC/nonblocking flags and kernel creator credentials are
checked. Healthy runs now have ten packets rather than nine; the shared parser's
five-FD,32-byte data and128-byte control bounds, five-second step bounds and
30-second outer bound are unchanged. Compatibility documentation registers this
private setup revision; mixed old/new prefixes refuse without fallback. This is
a cooperative local fixture, not authenticated endpoint or runtime custody against
arbitrary same-UID actors. Four independent pre-arm observers and eleven input
fault cases supplement the original five cuts and healthy controls.

Three final default-parallel groups pass83 each, and MSRV passes83 with the same
pinned Rust1.94 helpers. Ordinary launcher/root/GNU/isolation regressions pass54,
first-exec11 and root contracts159. Windows portability, strict lint/check,
175 registered gates and local contracts pass; ship still refuses. The
[gate lifetime packet](../testing/performance/0.74/local-runs/diagnostic-signer-constructor-gate-race-20261015/README.md)
retains the old negative evidence, both fresh baseline successes, controlled EOF,
red ordering regression and final checks, with15 verifier tests on both platforms.

This removes the identified observer ordering gap without changing the raw guard,
normal launcher/preparer, pinned helper, policy or qualification inputs. It is
neither a kernel fix nor a cache-performance improvement. Selected normal
root/runtime still needs original-child launch and kernel mapping composition;
full normal-entry loss, protected handoff, authenticated worker start and all-writer
cessation remain before completion and server admission.

### Ordinary selected root execution before signer admission

The next hypothesis was narrower than a production launcher: can the normal
binary, rather than a cfg(test) helper, connect its selected four-image root to
the original fork, observe that same child's first exec and loaded runtime, and
terminate it before removing the root? The preparation baseline passed 15 tests.
A new normal-build `root-exec` test first failed against the old entry with exit 2,
establishing that preparation alone did not already provide this execution path.

The implementation retains the original sources and copied root, prepares all
descriptors and fixed arguments before fork, then uses only scalar syscalls and
stack storage in the raw child. The child arms/readbacks PDEATHSIG/SIGKILL and
checks its captured parent before requesting tracing and stopping. Parent cleanup
owns the original fork before pidfd acquisition can fail. TRACEEXEC and EXITKILL
are installed before the first resume, overlapping the parent-death protection
before it is cleared for the user-namespace transition. This is launch mechanics,
not a new credential issuer or authenticated worker-start capability.

Only the child enters a new user namespace, with the same nonzero UID/GID mapped
and setgroups denied. It changes root/cwd to the selected root, closes the outside
root descriptor, removes capability sets and enables no_new_privs. Its standard
descriptors point to the retained null device; the only post-exec extra descriptor
is a private nonblocking write pipe. The executable descriptor is CLOEXEC and the
original copied self is executed by execveat, not by searching a supplied path.
The parent retains the original pidfd and process directory and rechecks root,
namespace, identities, capabilities, sole task, FD objects and empty environment
at the relevant stops. No process is adopted by a caller-supplied PID or descriptor.

An exec-stop is not enough. A development unit initially assumed the synthetic
ELF declaration fixtures could not reach the kernel exec event; that assertion
failed. The kernel accepted their shape and stopped before user code, even though
they could not execute the actual Rust child shim. The corrected test separates
those facts: selected executable identity and an exact kernel exec event establish
the first boundary; real shim execution and the loaded mapping set establish a
later, still diagnostic boundary. The synthetic negative must refuse there.

The new normal maps reader has fixed bounds of 65536 bytes and 128 lines. It checks
ordered page-aligned ranges, private non-WX permissions, selected device/inode
identities, exact role paths and offsets within rounded image sizes. Initially
it requires the executable and loader; after dynamic loading it requires all four
selected images with executable mappings. Restricted stack/vDSO and other known
pseudomaps are treated separately. These checks describe kernel VMA metadata;
they do not attest loaded bytes, relocations or immutable runtime custody.

The real child only checks its mechanical context, writes the fixed 16-byte
`HC074ROOTEXECv1\n` marker and SIGSTOPs. The marker is not an authentication token.
If unexpectedly resumed, the child exits 93 without signing, running a workload
or executing another image. After validating the loaded stop, the parent never
resumes it: original-pidfd SIGKILL, exact terminal wait/reap/ECHILD and private
stream EOF must precede the final root recheck and fixed-object cleanup.

The extra-writer control explains why these are separate conditions. Retaining
another writer to that exact pipe allows the original child to be killed and
reaped while EOF remains unavailable. Drop then explicitly preserves the root.
This demonstrates a conservative cleanup condition for this private channel,
not P2/P3 revocation of every possible writer. Mode 0555, direct-child death and
one channel's EOF still cannot replace independent custody or original cgroup
and descendant evidence.

Final targeted tests passed 25/25 in three default-parallel current-toolchain
runs, including one after source commit. Each group includes a real normal launch
and four further independent sequential owners, with the parent context unchanged.
The unit group includes 4096 seeded map mutations, status duplicate/identity/capability
refusals, phase replay and original SIGKILL/drop at initial, rooted and exec stops.
Old rooted/loss regressions passed 83/83, and another 50 launcher, GNU, isolation and
first-exec checks passed. Static contracts passed 160; the registry remains 175.
Rust 1.88 also passed 25 tests, but its observed extra-libm normal image explicitly
refused the unchanged graph. That result does not claim MSRV runtime-launch support.

There is a retained negative result on the final code. While the separate 83-test
group was running, two new process tests exceeded the unchanged five-second outer
guard; the process group finished 4/2, after units 18 and CLI 1 passed. The broad
group itself passed. Later targeted success used the same default test parallelism
and deadline, with no widened acceptance predicate. The overlap is known, but CPU
or scheduler attribution was not traced, so load robustness remains open. Killing
the owner at that outer deadline may leave its temporary root; a normal Drop
success is not crash-cleanup evidence. The failed capture is kept alongside the
positive repeats, not converted into an admission claim.

Source `5ca2cf2ca28f6f39a010a205e9ab649e91ebe977` and its
[local evidence packet](../testing/performance/0.74/local-runs/diagnostic-signer-owned-root-exec-20261015/README.md)
pin 12 changed files, 13 unchanged sources, frozen inputs and 24 captures. The packet
also retains the initial marker-size, inventory, synthetic-oracle, lint and format
failures. Its unsigned verifier checks consistency and source Git objects; it is
not an execution attestation. No cache-performance numbers are inferred from these
correctness tests, and product cache code is unchanged.

The remaining composition problem is concrete: the normal restriction installer
reads procfs, whereas this selected root deliberately contains no procfs. This
probe stops before that installer; it does not silently weaken policy, inherit
host-proc directory handles into the child or claim restrictions were installed.
Composing the genuine installer is the next isolated step, followed by full
ordinary-entry creator-loss faults and protected handoff. Authenticated original
start, real credentials, all-writer cessation, completion/later cells and server
qualification remain closed.

### Installing unchanged restrictions in the ordinary selected root

The next hypothesis concerned composition, not a new syscall policy: could the
ordinary selected-root launcher use the existing restriction installer without
mounting procfs in the child or passing host-proc descriptors into it? Preparation
and root-exec alone could not answer that. Their baseline group, combined with
ordinary child/policy regressions, passed 37 tests; a new root-native invocation
first failed against the old binary with exit 2.

The implementation separates the syscall-mutating core from context verification.
The original ordinary entry retains its procfs preflight and post-install checks;
both entries use one private installer and the existing native/x32/i386 exercises.
The selected root gains no procfs or extra files. Its parent already owns the
original fork/pidfd, selected images, namespace and process directory, and checks
the real first exec and loaded mappings. It retains the original environ and FD
directory before no-dumpability, in the parent only. Those readers keep their
identity/flags/procfs checks after ND0, without a fresh-open fallback.

The parent's new proof is actual syscall observation, not another readiness claim.
At the real seccomp entry it reads the child's 16-byte sock_fprog and all 512 policy
bytes from the previously validated stack range. Checked spans and aligned ptrace
reads bound access. The complete bytes must match the existing 64-instruction
policy, and mode FILTER/flag TSYNC must succeed. The fixed eight-pair entry/exit
sequence includes core limits, no_new_privs, filter installation, SET_DUMPABLE(0),
the installed marker and stop. Unknown calls, order, ABI, results or stops refuse.
In particular, descriptor mutation, fork and a second exec cannot hide inside
the accepted sequence. Kernel status must show exactly one additional filter.

A proposed extra check failed for a useful reason. After successful TSYNC and
SET_DUMPABLE(0), PR_GET_DUMPABLE returned EPERM. The unchanged prctl tail permits
the disabling operation but denies the query as well as restoration. Allowing
the query would change the policy being tested. We removed the new readback and
retained observed SET_DUMPABLE success, exact BPF, filter-count increment and the
existing restoration-denial exercise. The failed trace and preregistration
correction remain evidence; there is no successful direct-query claim.

Only after installed-state validation does the parent resume the fixed denial
exercise. Native checks all 20 existing EPERM probes and must exit 0 with its
distinct done marker. x32 and i386 must die with coreless SIGSYS and no done
marker. Loaded, installed and done messages are fixed 16-byte diagnostic data,
not authentication tokens. Original reap/ECHILD and pipe EOF still precede root
cleanup. Two further fixed modes exercise original-pidfd SIGKILL and ordinary Drop
from the validated installed stop. They demonstrate child cleanup, not loss of
the original creator or P2/P3 revocation of every writer.

Final current-toolchain tests passed 43/43 in three default-parallel groups, two
after source commit. The six new units include real original-child ptrace memory
reads and 256 seeded BPF mutations in a private pre-exec stack fixture. Only that
test fixture writes child memory; normal observation is read-only. Separate old
rooted/loss/first-exec/launcher regressions passed 121 tests. Static contracts passed
161 and the gate registry remains 175. MSRV passed 43 tests, with all five new
normal root modes explicitly refusing its extra-libm runtime graph. Its successful
test summary must not be read as admission of that unsupported runtime shape.

Strict lint initially hit an internal diagnostic-rendering panic. The available
capture is explicitly incomplete. A short-format retry exposed two code issues;
after correction, ordinary-format strict Clippy passed too. None of these results
required a new compiler, widened timeout, policy exception or lint suppression.
The prior overlapping-broad-load timeouts remain pinned; targeted success does
not resolve load robustness or prove root cleanup after owner SIGKILL.

Source `3c774c88ba99a2ea82d2d0840bdebdab97a0173f` and its
[evidence packet](../testing/performance/0.74/local-runs/diagnostic-signer-owned-root-isolation-20261015/README.md)
pin changed sources, unchanged policy/fixture sources, frozen inputs and the
negative observations. The verifier also compares the original entry's pre/post
checks and native exercise with the extracted implementation. This is local
consistency evidence, not execution attestation or a cache-throughput measurement.

The result closes ordinary root/restriction composition for these diagnostics.
Next come full ordinary-entry creator-loss faults and protected one-shot handoff.
Independent runtime custody, real credentials, authenticated worker original-start,
all-writer cessation, completion/later cells and server qualification remain closed.

### Losing the ordinary root creator without confusing cleanup with protection

Installing restrictions and killing the worker itself left a different question
unanswered: what happens when the original creator dies? A passing Drop path
cannot answer it, because Drop is precisely what SIGKILL prevents. We preregistered
a bounded experiment at five owned stops, rather than describing it as full crash
recovery. The ordinary 43-test baseline passed; the old binary then refused the
new loss mode with exit 2. That red is retained before implementation results.

The normal diagnostic now starts from a fresh sole-main nonroot monitor. It creates
the selected four-image root and retains its original objects before a fixture-only
creator fork. The creator uses the existing ordinary Owner, raw worker child,
namespace, first-exec, loaded-map and unchanged-policy methods. This is normal-build
composition, but the outer creator is not a fresh exec of the creator CLI. We did
not turn a PID/path supplied by a report into a general process-adoption API.

A private seqpacket setup transfers the original worker pidfd and stream reader.
Kernel SCM_CREDENTIALS must identify the expected creator PID/UID/GID. Procfs must
show that creator's sole original traced child, the pidfd must refer to that child,
and pipe identity must match its actual initial FD3. An O_PATH metadata handle
checks the FIFO without opening it for I/O. Received rights become owned RAII
objects before rejection; both payload and ancillary truncation refuse. Tests
detect leaked duplicate writers by requiring EOF on the original reader. These
checks bind this fixture's objects, not independently authenticated runtime custody.

The loss monitor signals only the original creator pidfd and keeps its endpoint
open until exact creator SIGKILL and reap. This preserves the earlier lesson that
channel EOF can precede creator death; EOF is not used as that death oracle. The
monitor temporarily becomes a subreaper and requires exact adopted original-worker
SIGKILL via WNOWAIT, reap/ECHILD and stream EOF. Only then, after explicit cleanup
of its unrelated pause child, can it recheck and remove the original root. Unknown
quiescence preserves the tree. Emergency original-child cleanup cannot convert a
failed proof into a success, and the monitor restores its own subreaper state.

The strongest control is intentionally a survivor. At installed loss we first
establish a kernel group-stop, distinguishing it from SIGSTOP delivery via the
GETSIGINFO/EINVAL check. A matched control removes EXITKILL at that same stop.
After creator death it must remain live, adopted, untraced and stopped. Only an
explicit later worker kill cleans it up. It passed as a **negative control**, not
as automatic termination. Two healthy acknowledgements instead let the original
creator terminate/reap its worker. Final markers label loss, healthy and negative
separately; a generic "child eventually died" would lose the attribution.

The five positive stopped cuts—initial after options, rooted, first exec, loaded
and installed—passed in three current-toolchain 52-test groups, two after source
commit. Each group also passes the healthy/negative controls and four additional
fresh sequential installed-loss monitors, with unchanged caller context. Nine new
units cover fixed frames, credentials, descriptor cleanup, original pipe identity,
status shapes and conservative tree preservation. The final source leaves the
ordinary owner/raw child, exact policy and earlier fixtures unchanged. No wait,
image graph or proc-text bound was widened to obtain acceptance.

MSRV passes 52 tests while explicitly refusing all eight root-loss modes for its
extra-libm image. That is a successful refusal test, not accepted runtime support.
Separate old regressions pass 121; static contracts pass 162 and the gated registry
remains 175. Current-toolchain strict Clippy passes on Linux and Windows, as do
current/MSRV checks. MSRV Clippy could not run because its component is absent;
the failed invocation is retained, not reported green or hidden behind a toolchain
change. Local contract passes while require-ship still refuses. No throughput,
allocation or latency improvement is inferred from these lifecycle test counts.

Source `1f399e0a7040d2a0733825aed2fd13fc6caf85a4` and the
[local evidence packet](../testing/performance/0.74/local-runs/diagnostic-signer-owned-root-loss-20261015/README.md)
retain baseline/red/development/final captures, frozen inputs, unchanged sources
and earlier overlapping-load failures. The unsigned verifier checks historical
consistency and rejects promotion of the survivor or MSRV refusal. It is not an
execution attestation. Those earlier five-second overload timeouts remain open.

The result supports automatic termination after creator process loss at these
five **stopped** boundaries. It says nothing about pre-arm loss, a running worker,
creator-thread-only death, fresh creator CLI exec or hostile same-UID custody.
The next isolated step is protected one-shot dummy handoff through the ordinary
root/policy path, followed by the remaining loss boundaries. Real credentials,
authenticated worker original-start, P2/P3 cessation of all writers, completion and
later cells, server rehearsal and release qualification remain separate gates.

### One shot dummy transfer through the ordinary root

The next question was whether the ordinary selected-self launcher could carry
the existing fixed dummy protocol after its actual root, exec, loaded mappings
and restriction checks. The previous handoff proof used a separately compiled
test helper; the stopped-creator-loss proof did not transfer anything. Reusing
those results as evidence of ordinary transfer would skip the composition being
tested. We retained the 52-test baseline and the old binary's new-mode exit2 red.

The added profile replaces only its private FD3 pipe with an unnamed nonblocking
CLOEXEC seqpacket pair. The raw worker fork path stays unchanged. The parent
drops its child-end duplicate immediately and retains original endpoint objects.
Existing pipe modes keep their own behavior; no new root file or procfs mount
appears. A normal private child mode performs the same context prerequisites
before any fault action, then the same fixed policy installer. Neither the
library nor a production server gains a credential or process-launch API.

Startup observation is bounded to 256 syscall pairs and a finite loader/context
query set. The parent's original trace rejects early channel use, FD replacement,
another exec, fork/thread creation and root/namespace changes before dispatch.
The loaded marker is checked from stack; actual TSYNC/BPF/ND0 installation remains
the existing eight-pair sequence. Before ND0, original FD3 inode is checked.
After ND0, parent-retained live inventory and the strict accepted trace exclude
replacement without attempting a fresh forbidden FD-object open.

READY alone does not release the dummy. Its exact original sendto call, flags,
stack bytes, return16 and received record must first match. The next original
syscall must then be the exact bounded recvfrom entry. At that entry the parent
rechecks full context/maps and the empty queue, latches its only attempt, and
sends public `HC074-DUMMY-v001`. It checks the original read result and stack
bytes, exact ACK, close3 and stdio-only inventory, then original zero exit,
reap/ECHILD and EOF before root validation/cleanup. A wrong, duplicate or lost
ACK cannot make that attempt retryable. A failed send is still attempted even
when zero successful sends are counted.

Two development failures changed the implementation, not the acceptance budget.
First, the ordinary loaded marker lived in rodata and correctly failed the
stack-only reader. Only the new socket profile's marker moved to stack. Second,
an implementation that reread full root/context around every individual call
exhausted the unchanged five-second outer guard. A diagnostic trace identified
repeated full-context scans before completion; it was not a product benchmark.
The correction was recorded before the next candidate: full checks at installed,
the exact read entry and after close, with original pidfd/endpoint, exact native
entry/exit and stack checks between them. That intervening sequence admits no
FD/root/process mutation except final close. No old Owner check was removed and
no timeout, graph, policy, compiler/profile or release threshold was widened.

Failure handling also needs transport-specific care. MSG_TRUNC preserves actual
datagram length for the existing 48-byte drain bound. A zero-length record from
a live peer is not EOF, so HUP is required. Under this fixed observed writer's
maximum four records, five bounded reads cross EOF; data after a zero refuses.
This is not a general reader for an untrusted unlimited datagram stream. Failure
to establish original terminal/stream quiescence still preserves the root.

The 25-mode process matrix distinguishes successful one-shot transfer from
malformed/duplicate readiness, short/oversized records, wrong FD, early use,
leaked FD, ACK faults, close/exit, sendmsg, replacement, rodata, withheld data,
failed send and phase replay. SIGKILL and ordinary Drop controls at installed,
ready and delivered stops prove child cleanup, not loss of the creator at those
boundaries. Eight new units cover closed selectors, no-retry/event budgets,
original socket flags, queue/drain behavior, finite startup calls and full-width
argument mutations. Strict lint required moving the unit module after the
implementation; no lint allowance was added.

Three final current-toolchain groups pass 61 tests each, two after source commit:
41 root units, five ordinary child-entry, seven policy, one root CLI and seven
process tests. Each process group actually exercises all 25 handoff modes and the
previous ordinary root/policy/loss controls with default parallelism. MSRV also
passes 61 tests but explicitly refuses all 25 handoff modes for its extra-libm
image; that is not supported-runtime admission. Windows current/MSRV portable
groups pass eight each, current/MSRV checks pass, and strict current Clippy is
green on Linux and Windows. MSRV Clippy remains unperformed: the installed
component inventory does not contain it. The local contract remains non-promotable
and require-ship still refuses; no full release gate or numerical claim follows.

Separate old regressions pass 121 tests: rooted/loss 83, first-exec 11, private
policy parsers three and old owned-probe/GNU/CLI/process 24. The broad rooted
group starts after the final ordinary process repetitions, not as an overlapping
load-robustness experiment. Static contracts pass 163; the gate registry stays 175.

Source `c30c0bc4cc417e44a38d7482dafaf651f3df3302` and the
[local packet](../testing/performance/0.74/local-runs/diagnostic-signer-owned-root-handoff-20261015/README.md)
retain the feasibility correction, negative captures and final verification.
The next separate experiment is creator loss before/during/after this transfer.
The earlier five stopped cuts do not establish that result automatically.
Real credentials, independent runtime/issuer/job custody, authenticated original
worker start, all-writer cessation, completion/later cells and server admission
remain separate. This protocol adds no throughput or allocation claim.

### Creator loss during ordinary dummy transfer

The ordinary transfer and earlier stopped-creator-loss results did not by
themselves prove that the same original worker would stop if its creator died
between READY, delivery and ACK. The next local experiment composes those paths
using public dummy bytes only. It retains the 61-test baseline and the unchanged
binary's exit2 refusal under the new scenario before implementation. The
[preregistration](../testing/performance/0.74/diagnostic-signer-owned-root-handoff-loss-preregistration.md)
fixes six boundaries: installed, READY received, original recv entry before any
attempt, original recv return with the dummy, ACK received and close return
before exit. Each boundary has a loss and a healthy continuation; installed
and delivered also have matched no-EXITKILL survivor controls.

Socket ownership required a different check from the old pipe experiment.
Opposite ends of a socketpair have different inodes. Comparing the monitor's
reader to the child's endpoint as if they were a pipe would reject the real
pair or encourage an invalid identity shortcut. Instead, the monitor creates
the pair and retains the original Root before its fixture-only creator fork.
The same private Owner construction body receives this fixed transport, without
changing the raw worker child, root/exec transitions, policy or child protocol.
The monitor compares the transferred parent endpoint to its original parent,
and initial child FD3 O_PATH metadata to its original peer. Original pidfd,
sole-child, parent/tracer and per-packet kernel credential checks remain.

The monitor must close its extra child-end copy before allowing G. Otherwise
the worker could die while that copy prevents EOF, making lifetime evidence
incorrect. Conversely, the parent endpoint must stay live until creator terminal
and reap; early peer closure must not cause the worker's apparent automatic
death. These are object-lifetime requirements, not optimizations. Received
descriptors retain CLOEXEC, socket type/domain and nonblocking read/write checks.
No caller can choose a process, endpoint, root, payload or continuation pointer.

For each passing loss the monitor sends SIGKILL only to the original creator
pidfd. It observes exact creator terminal/reap, then adopted original worker
SIGKILL through WNOWAIT/reap/ECHILD, bounded socket EOF and unrelated original
child survival before original root cleanup. Installed and delivered pairs
first establish a genuine group-stop: delivered queues SIGSTOP at the original
recv exit, reaches its signal-delivery stop, reinjects SIGSTOP and requires
GETSIGINFO EINVAL. The matched no-EXITKILL controls must instead survive as
adopted, untraced State T. Their later explicit worker kill is cleanup and must
never be counted as automatic-death proof. The other four loss points are ptrace
stops, not evidence for a freely running worker.

Healthy H resumes the original protocol from each boundary, including the
observed recv-entry stack pointer, and requires ACK/close/zero exit and ordinary
reap. The sticky one-attempt latch remains unchanged. Before delivery, send and
attempt counts must be zero; after verified receipt they must be one. New units
reject wrong step/attempt/send/close combinations, foreign/swapped/equal socket
endpoints and altered flags. Existing exact frame-bit mutations cover all 14
new modes. The private frame stays version1/16 bytes with two setup descriptors,
the same gates, four-packet budget and five-second waits.

The first isolated 14-mode matrix passed. Adding it as a separate test increased
the number of concurrent ordinary root fixtures, and the old handoff test then
hit its unchanged five-second guard. No scheduler trace establishes the cause.
That red remains in the packet. Before the next candidate we recorded a fixture
scheduling correction: run the 14 cases after the old 25 in their existing
process test, reusing its original Image. This preserves the baseline number of
concurrent fixtures and default test parallelism without adding lifetime locks,
ignored tests or a larger timeout. It controls added fixture load; it does not
fix or close overlapping-load robustness. A compile-time FixedProbe equality
error was fixed with pattern matching, not by changing the shared enum.

Two corrected targeted groups pass 63 tests each, the second after source commit
`97d9304f162a548ba0181cb4334960598b5f1cac`: 43 root units, five ordinary
child-entry, seven policy, one CLI and seven process tests. The process matrix
executes all 25 old handoff cases followed by the 14 new controls. The
[local packet](../testing/performance/0.74/local-runs/diagnostic-signer-owned-root-handoff-loss-20261015/README.md)
retains failures, repeated checks and source consistency separately. Its baseline
raw capture contains only the process tail; the complete baseline result was
observed but is not represented as a complete saved transcript.

Current Linux/Windows strict Clippy and current/MSRV checks pass. MSRV's 63-test
group explicitly refuses all 14 new extra-libm modes rather than claiming runtime
support; portable Windows current/MSRV groups pass eight each. Static contracts
pass 164 and all 175 discovered gates remain registered. The local performance
contract is non-promotable and require-ship still refuses. The MSRV lint gap is
now more precise: its Linux component is absent, while Windows strict Clippy was
executed and found seven existing uninlined_format_args errors in the unchanged
macro crate. Linting that package alone reproduces them. Their original three
source blobs are pinned in the packet; no unrelated formatting or lint allowance
was mixed into the lifecycle implementation. This is not an all-checks-green
or full release-gate result.

After the second ordinary group completes, the separate old regressions pass
121 tests: 83 rooted/loss, 11 first-exec, three policy parsers and 24 old
owned/GNU/CLI/process checks. They do not overlap the ordinary process guards.
Historical Git comparison preserves 22 original function bodies, including raw
child, original waits, credential channel, pipe binding/group-stop, fixed child
protocol, startup and the outer guard. Frozen qualification inputs remain byte
identical. These checks protect the existing paths without turning a local
dummy experiment into server or performance evidence.

The packet seals 32 raw captures and pins source/tree, frozen inputs and earlier
negative results. Its consistency verifier and seven mutation/unit tests pass
on Windows and Linux; final verifier transcripts are kept separately from the
sealed logs. These unsigned checks detect inconsistent records, not forged
execution. Neither a complete baseline transcript nor signed/clean-checkout
execution attestation is claimed.

The result closes only these six stopped public-dummy boundaries. It does not
interrupt the creator between the attempt latch and send, or between successful
send and resuming the original recv. Those uncertain-send windows are the next
bounded local experiment; permission recovery after creator restart is not
tested. It also does not
establish pre-arm, running or creator-thread-only loss, fresh creator CLI exec,
hostile same-UID custody or signed execution attestation. Trusted runtime/issuer/
job custody and credentials, authenticated original worker start, P2/P3 cessation
of every writer, completion/later cells and server admission remain separate.
No product candidate, expensive qualification or numerical performance claim
was enabled by these lifecycle checks.

### Creator loss between attempt send and receive

The six stopped transfer cuts left two useful creator-side boundaries untested:
the attempt had been consumed but no send dispatched, or send16 had succeeded
while the original worker still waited at its recv-entry stop. The next local
experiment isolates those two boundaries without adding a product worker or
real credentials. The complete 63-test baseline is retained, and the unchanged
binary refuses the new scenario with exit2 before implementation. The
[preregistration](../testing/performance/0.74/diagnostic-signer-owned-root-send-windows-preregistration.md)
fixes two losses, two healthy continuations and two no-EXITKILL controls.

The state now distinguishes the attempt latch, the dispatch latch and successful
send count. Dispatch is latched before the syscall and never becomes reusable
after an error. This is an intention to dispatch, not proof that data reached
the peer; successful send16 is recorded separately. At the first cut the facts
are attempt1/dispatch0/send0. At the second they are attempt1/dispatch1/send1,
but the same original worker has not resumed recv. Healthy continuation from
the first cut dispatches once; from the second it only resumes the original
recv, checks received stack bytes and completes the existing ACK/close/zero-exit
trace. There is no second attempt or dispatch, and no durable permission-recovery
claim follows from these per-lifetime flags.

The original monitor, Root, socket pair, credential-checked setup channel and
worker pidfd are reused. Passing loss sends SIGKILL only to the original creator
and requires original worker SIGKILL/WNOWAIT/reap/ECHILD, then socket EOF and
unrelated original child survival before root cleanup. Raw child, policy,
installer, full-context boundaries, startup256 and protocol16 budgets stay fixed.

Unread data makes the second loss materially different. A preliminary local
seqpacket check found one ECONNRESET when a peer closed with the dummy unread,
followed by actual zero reads with HUP. Without pending input there was only
zero/HUP. Treating that reset as EOF would hide an unproved terminal stream;
rejecting every reset in the new disposition would also prevent completing
this exact loss experiment. The implementation therefore leaves the old
48-byte/five-read drain body unchanged and requires one reset prefix only for
the new sent-loss cut. It then requires zero without queued extra data and the
old mandatory EOF/HUP proof. Unexpected reset, other errors or live zero records
still refuse; unknown quiescence preserves the root. This is not generic reset
tolerance or a widened drain/timeout budget.

The no-EXITKILL controls deliberately do not group-stop or kill the worker.
After creator loss the same original child runs untraced. With no input it
exits74 without ACK. With queued input it emits ACK16 and exits0. The sent
control's ACK and terminal EOF are observed, but close is not ptrace-observed.
Both outcomes are negative counterexamples, not healthy completion. In
particular a plausible ACK and zero exit can coexist with creator loss when
lifetime enforcement is missing; those signals alone cannot authenticate
original start or prove that every writer has lost authority.

All six isolated process controls match these expected outcomes. Four new units
cover dispatch/replay/EPIPE, reset versus EOF, exact single ACK versus malformed
or duplicate records, and unknown modes/live peers. Existing cut-state and
frame-bit mutation tests cover the new facts and private cuts11/12/mode3. The
new cases follow the old25/14 matrices in their existing process test; default
parallelism and the five-second per-process guard remain unchanged. The initial
development targeted group passes 67 tests; final source repetitions and
historical function/frozen-input checks are retained separately in the
[local packet](../testing/performance/0.74/local-runs/diagnostic-signer-owned-root-send-windows-20261015/README.md).

The final implementation is `541c752b072a755774493e93bbecb85b4f1746b5`. Two
post-commit current-toolchain groups pass 67 each: 47 root units, five child-entry,
seven policy, one CLI and seven process tests. Each process matrix executes the
old25 and old14 cases before all six new controls. MSRV passes 67 with explicit
six-mode extra-libm refusal, not supported-runtime admission. Windows portable
groups pass eight each; current/MSRV checks pass on both platforms and current
strict Clippy is green on Linux/Windows. Static contracts pass 165 and the registry retains 175 gates;
formatting/doc-check and the non-promotable local contract pass, while require-ship
still refuses. The repeated Windows MSRV lint retains seven unchanged macro
errors; its Linux component remains absent. Historical comparison verifies 42
original function bodies, including old drain, raw child, credential channel,
frame codec and outer guard, plus separate sticky-dispatch/prefix-order checks.
This is not a full release gate or an all-checks-green receipt.

The old regressions then pass 121 separately: 83 rooted/loss, 11 first-exec, three
policy parsers and 24 old owned/GNU/CLI/process tests. Neither broad group
overlaps the ordinary five-second guards. Frozen qualification inputs and prior
negative captures remain unchanged. No product performance measurement was
collected by this lifecycle experiment.

The packet seals 32 complete raw captures and pins source/tree, frozen inputs
and earlier negative results. Its verifier and eight mutation/unit tests pass
on Windows and Linux. Separate final verifier transcripts are unsigned
consistency checks, not execution attestation or a production-start receipt.

These are two fixed boundaries, not arbitrary-instruction or in-syscall loss.
Restart/permission recovery, fresh creator CLI exec, pre-arm/running/thread-only
loss, hostile same-UID custody, trusted credentials/runtime/issuer/job custody,
authenticated original worker start, P2/P3 cessation of every writer and later
cells remain separate. Earlier parallel-fixture timeouts and MSRV macro lint
failures remain recorded rather than waived. No server, qualification, product
workload or throughput/allocation claim is opened by this local experiment.

### Macro diagnostics under the minimum Rust version

The send-window packet left a reproducible code-quality failure: Rust1.88 strict
Clippy rejected seven format expressions in the procedural macros, although
the current-toolchain check passed. Repeating the old command reproduced all
seven errors. The original 76 macro tests passed, but most diagnostic assertions
checked only substrings and did not cover every affected branch.

Seven new tests first fixed the exact existing messages for duplicate cacheable
expressions and segment lists, duplicate query dependency lists and types,
missing list commas, empty dependency lists and duplicate prepared-policy types.
All 83 tests passed before the implementation changed. The implementation then
replaced only seven positional format arguments with implicit captures. Parser
conditions and error spans remain unchanged; there is no lint suppression or
new accepted input. The same 83 tests pass on Windows and Linux with Rust1.88
and Rust1.94, and the macro package passes strict Windows MSRV Clippy.
The Linux current-toolchain runtime and DB consumer suites also pass their
compile-pass/compile-fail snapshots without updating expected diagnostics.

The wider Windows MSRV lint now reaches three existing format diagnostics in
`grid/conditional.rs`, previously hidden behind the macro failure. They remain
a failed gate, not a successful cleanup receipt. Linux MSRV Clippy remains
unperformed because that component is absent. Current-toolchain wider checks
and strict Clippy pass on both platforms. The
[local evidence](../testing/performance/0.74/local-runs/msrv-macro-diagnostics-20261015/README.md)
retains the original red output, the test-first diff and the subsequent results;
the earlier lifecycle packets keep their original source identities and failures.

This is a compiler-compatibility cleanup, not an allocation or throughput
improvement. It does not requalify a product candidate or advance authenticated
original-start, P2/P3 writer cessation, completion or later lifecycle cells.

### Conditional diagnostics and feature aware example builds

The next MSRV cleanup preserves three conditional error messages rather than
changing consistency or fencing. The baseline strict lint reproduces three
`uninlined_format_args` errors; the original conditional suite passes six tests
and leaves its existing chaos marker ignored. Three new characterization tests
then pass on the old implementation before changing any `write!`. They fix exact
message bytes across consistency enum values, key counts including `usize::MAX`,
missing/present owners, Unicode keys, escaped session text and `u64::MAX`.
Synthetic error construction tests formatting, not a decision to reject every
consistency level. Only the three interpolations change.

Forty focused tests pass on each Windows/Linux and Rust1.88/1.94 combination:
conditional writes, tombstone ordering, lock leases, reentrancy and consistency
levels. The same existing chaos marker remains ignored. The Linux Rust1.88
Clippy component is installed locally, closing the earlier missing-tool gap.
This does not retroactively change the preceding packets' results.

The package-wide check exposes another independent build failure: the memory
compatibility example imports durable types while Cargo tries to compile it
without the optional durable feature. A separate manifest-only commit declares
`required-features = ["durable-value-store"]`. Default features stay empty and
the example source and compatibility workload stay unchanged. Default all-target
checks now skip that incompatible target through Cargo's normal feature rule.
An explicit request without the feature fails with Cargo exit101 and the required
feature message; explicit durable-enabled example checks and strict Clippy pass
on both platforms/toolchains. Those paired checks retain coverage of the example
instead of presenting its default exclusion as proof that it compiled.

The package default all-target checks and strict lint pass in all four lanes.
Wider MSRV tooling lint still fails: Windows reaches three loadgen format
expressions, while Linux first rejects a nested else/if in the root-handoff
diagnostic. A separate warning-mode inventory is diagnostic only, never a green
strict-gate receipt. The
[local packet](../testing/performance/0.74/local-runs/msrv-conditional-diagnostics-20261015/README.md)
retains those failures alongside the successful focused checks and build
metadata comparison. These compiler/build changes add no throughput or allocation
measurement and do not establish authenticated original-start or writer cessation.

The completed warning inventories identify eight unique remaining locations:
three loadgen formats, four xtask formats and one Linux-only supervisor branch.
Duplicate library/test diagnostics are not separate defects. This bounds the
next local cleanup for the selected targets; the inventory's warning-mode exit0
does not satisfy the preserved strict MSRV gate.

Post-commit Windows MSRV and Linux current repetitions each pass the same 40
focused tests. The 165 static performance-contract tests, 175-gate registry and
non-promotable local contract pass; require-ship still refuses. Wider current
strict Clippy is green on Windows/Linux, while the recorded MSRV failures remain
open. No full release gate or server qualification is implied.

### Closing the tooling lint inventory without changing lifecycle authority

The next local step addresses the eight observed sites, not another performance
hypothesis. Its baseline reproduces the strict MSRV failures before touching
the source. Seven sites interpolate already-present values in archive names,
CSV and in-flight diagnostics, canary/fast-suite messages and bootstrap dates.
The eighth is a nested else/if in the Linux-only private `loaded` trace routine.
The changes are split into a formatting/test commit and a separate trace-control
commit so their review and evidence are not conflated.

Five new characterization tests pass on the old formatting first. They pin
decimal archive names and hard-linked bytes, the full expected/observed CSV
header message, both missing-plan/canary messages, the existing 1680-second
ceiling message, and zero-padded dates. The ceiling is mutated only in a local
fixture; no real budget changes. After the seven implicit captures, Windows
Rust1.94 passes 60 focused tests. Four of those are pure client-surface checks;
the measurement smoke validator is explicitly skipped. The actual in-flight
mismatch branch is not driven by a measurement run. Its unchanged arguments and
message structure are established by the exact source comparison, not claimed
runtime coverage. Scripted external-tool tests are not a Redis benchmark.

The supervisor edit collapses only `else { if marker ... }` into `else if marker`.
The initial READY-write branch still has priority, followed by the same marker,
getpid/kill and startup checks in the same order. Whole-file comparison against
the baseline permits only that registered rewrite; other supervisor sources,
the 256-event budget, five-second guards and default process-test concurrency
are unchanged. The `loaded` body does change syntactically, so this is not a
claim that all historical function bodies or tooling source identities match.

After both edits, all-target/all-feature checks and strict Clippy for the selected
loadgen/xtask/supervisor packages pass on Windows/Linux with Rust1.88/1.94.
This closes the observed strict lint inventory without suppressing warnings.
It is neither an exhaustive workspace/feature matrix nor new Rust1.88 runtime
support for the ordinary selected-root profile. The preceding failed captures
remain historical facts under their original source IDs.

The first process group after the trace edit passes 66 tests but fails the
handoff integration test at its unchanged five-second guard, before its first
good-mode receipt. The preceding baseline passed 67. No separate heavy build is
deliberately overlapping this group, so the failure is not attributed to the
earlier compiler-overlap explanation. Its generic timeout does not identify the
stalled instruction. The complete negative is retained, not hidden by the green
compiler checks; baseline controls and later repetitions are separate evidence.

Restoring only the original branch passes the same 67-test group; restoring
the registered edit also passes 67. The latter includes actual public-dummy
handoff and stopped loss/healthy/no-EXITKILL controls, with unchanged default
parallelism and five-second guards. This makes the observed failure intermittent,
but neither the passing control nor the repetition establishes its cause. It is
not a load-robustness receipt or a waiver of the failed run.

The post-commit repetition also passes all 67. The packet therefore preserves
two original-trace successes, the trace candidate's initial 66/1 failure and two
later trace successes separately. The next diagnostic task is to identify the
intermittent stall without changing bounds or scheduling, before claiming robust
lifecycle behavior. It does not remove the separate authority and
all-writer cessation requirements.

The result is build-quality compatibility and more precise regression coverage,
not a measured allocation or throughput gain. The static 165-test contract and
175-gate registry pass, while the local contract stays non-promotable and ship
admission still refuses. The
[local packet](../testing/performance/0.74/local-runs/msrv-tooling-trace-cleanup-20261015/README.md)
separates baseline, test-first, compiler, process and structural checks. Unsigned
hash consistency does not attest execution, and new source commits cannot reuse
old qualification receipts as if they described the new tools. Authenticated
original-start, trusted runtime/issuer/job custody, P2/P3 all-writer cessation,
completion/later cells and server admission remain the substantive W11 work.

### Bounded fault output identifies expensive probe verification

The next experiment keeps the five-second guard and asks what happens before
it kills the probe. Another original-source group passes 67, so a successful
repeat still cannot explain the earlier failure. A new controlled test supplies
public bytes to an owned cat process but keeps its input open. The old guard
kills/reaps that original process and loses its output; the characterization
test fails on the missing terminal/output facts before any implementation change.

Only the test-side guard is extracted. Its deadline, five-millisecond polling,
kill/wait order and the original seven process fixtures remain unchanged. After
kill/wait, each owned output read end becomes nonblocking and supplies at most
4096 bytes/eight reads. EOF, would-block, capacity/read budget and IO/setup errors
are distinct: a live writer cannot turn diagnostic capture into an unbounded
read or an invented EOF. Nine tests cover the real SIGKILL, exact successful and
nonzero output, live writers, byte limits, repeated EINTR, failures and seeded
binary prefixes. They pass on Linux Rust1.88 and Rust1.94. The report uses the
original Child wait, not a numeric PID recheck after release that could address
a reused process. It makes no claim about unobserved descendant writers.

Parent-only timing instrumentation is a separate source overlay, never a green
canonical receipt. Two stage runs complete in about 4.75/4.90 seconds wall with
4.57/4.69 seconds parent CPU; the normal stderr-empty assertion rejects their
extra output. A separate leaf overlay then catches a real guard timeout after
completed and cleanup-begin. Its last frame records 4.51 seconds in completed
SHA-256 calls out of 4.71 seconds parent CPU. Another leaf run completes with
184 hashes over 282969552 input bytes, 4.43 seconds hash CPU and 4.61 seconds
parent CPU. Those are hashed public runtime-image bytes, not copied bytes or
product payloads. The counters identify this local verification owner; the last
frame does not identify the exact interrupted instruction or prove completion
of cleanup, nor retrospectively explain the old uninstrumented timeout.

The conditional candidate changes only the private root materializer's digest
implementation to the already pinned ring 0.17.14 SHA-256 backend. It does not
cache hashes or skip a source, copy, binding, metadata or cleanup check. Before
that edit, known empty/abc vectors and seeded comparisons against the old sha2
reference pass across sixteen padding/block/page/image boundaries up to eight
MiB. They pass afterward as well. The complete 32-byte digest is preserved;
dependencies, lockfile, compiler profile, runtime graph and limits stay fixed.
The probe is binary-private: embedded/native/RESP product paths are untouched.

All temporary stage/counter code is removed before canonical checks. Three
current-toolchain Linux groups pass 68 afterward: the original 67 plus one pure
hash-equivalence test. Their unchanged seven-process-test target reports about
16.83/16.88/17.02 seconds versus the baseline's 170.81. These are local test-run
observations, not paired throughput evidence, a portable speedup ratio or a
product performance claim. Different executable identities also change the
self-image bytes being hashed. Strict package checks/lint pass on Windows/Linux
Rust1.88/1.94. The MSRV group passes its tests while still refusing the extra-libm
normal root image; that refusal is not new runtime support.

One leaf-attribution attempt stops earlier with EAGAIN in the unchanged
SCM_RIGHTS cleanup unit. It supplies no handoff sample and remains an unexpected
negative. Concurrent fork inheritance of a writer is a hypothesis to test with
owned process/FD gates, not a proven explanation or a reason to add retries or
relax EOF. Later green groups do not waive it. The
[evidence packet](../testing/performance/0.74/local-runs/root-guard-attribution-20261015/README.md)
keeps raw negatives, temporary overlays, test-first snapshots and separate source
identities. Next work must address that fixture/lifetime uncertainty alongside
trusted runtime/issuer/job custody, authenticated original-start and P2/P3
all-writer cessation; server and release qualification remain closed.

The first global registry check also catches the missing registration of the
new Linux-only guard test target. A separate entry fixes that omission without
changing existing gates or CI workflows. The registry now covers 176 gates and
all six registry validator tests pass; the 165 performance-contract tests pass
again. Local admission stays non-promotable and require-ship still refuses.

### An inherited writer can invalidate an immediate EOF oracle

The next local experiment separates a received-descriptor leak from a fixture
lifetime error. The unchanged-source root group first passes 68 tests. That does
not explain the retained EAGAIN: other unit tests fork while the cleanup test can
have an open writer. CLOEXEC is an exec boundary, not a fork boundary, so the
cleanup test cannot infer sole writer ownership merely from its own Rust handles.

A controlled child reproduces that mechanism. The test queues two writer rights,
forks, and requires the child to close the unrelated endpoints, verify CLOEXEC
on its inherited writer and acknowledge custody. The parent drops its writer and
rejects the rights packet using the unchanged Channel. Applying the old immediate
EOF assertion now fails with EAGAIN. The separate exec wrapper propagates that
child failure; it does not promote a failed inner test to a passing outer test.
The passing characterization instead requires WouldBlock while that acknowledged
child remains nonterminal. A fixed release gate lets it close the writer and exit;
the parent requires the original pidfd exit/wait/reap before demanding EOF. No
sleep, retry or elapsed-time assertion substitutes for those logical boundaries.

The correction isolates only the two received-rights EOF fixtures in fresh exec'd
copies of the unit-test binary. Each selects exactly its existing test with a
private child marker and empty environment. Exec removes incidental CLOEXEC
writers before the fixture starts; other outer tests still run concurrently.
The existing five-second process guard, immediate EOF assertions, credentials,
ancillary truncation rejection and all Channel code remain unchanged. There is
no new normal-build executable mode or production authority input.

Isolation must not hide a real retained writer. A separate control keeps the two
successfully received rights alive: the same EOF assertion deliberately fails.
After characterization, the control requires WouldBlock with both rights and
again with one, then EOF after the final close. Another test rejects an exec
which succeeds with zero selected tests: exit zero alone is not proof that the
intended check ran. The final root groups pass 71 on Linux Rust1.94 and Rust1.88,
including these three additions; nine guard fault tests pass on both. The MSRV
normal-build image still takes the explicit extra-libm refusal path.

The first static contract run rejects the inline test helper because the entire
loss source file forbids Command construction. The helper moves to the existing
separate root-test file, leaving a thin cfg(test) delegation at the call site.
The forbidden-token guard and every production-source condition remain unchanged;
the same focused static check passes afterward. This is a placement correction,
not a new exception to the launch contract.

The [evidence packet](../testing/performance/0.74/local-runs/scm-rights-eof-isolation-20261015/README.md)
retains both deliberately red controls, the test-first diff and the original
historical negative by reference. The controlled reproduction establishes a
possible mechanism, not the identity of the writer in that historical run.
This test-only correction also does not establish production P2/P3 all-writer
cessation. Trusted runtime/issuer/job custody, authenticated original-start,
completion/later cells and server admission remain separate W11 work.

### Keeping controller verification inside the original fence

The next P2 step addresses an ownership gap: controller provenance and common
campaign exclusion existed as separate original guards. Checking one and then
discarding it before taking the other is not a retained preflight for a later
write. The local baseline passes seventeen OIDC cases and eleven empty-fence
cases. A new positive composition test first fails on the missing binding method.

The new opaque preflight exclusively borrows the original OIDC guard and owns
the already acquired original empty fence. That OIDC owner still retains its
original builder/controller/worker/kernel/configuration/authority chain. The
composition acquires no new journal or lock and accepts no replacement peer,
time, request or token. It checks fence, controller, fence, controller, fence;
the existing controller verifier obtains freshness from the same original kernel
context, while the existing fence verifier checks the same objects and empty
storage. No verification condition is removed or cached away.

A failed check preserves its typed error, stops the bracket and refuses both
chains. Constructor failure owns the refused fence rather than releasing its
lock as a side effect of returning an error. Later failure also retains that
original lock until the preflight is dropped. A refused repeat performs no more
observations; restoring public files cannot repair the guard. Healthy drop
releases only the fence and leaves the controller inputs healthy. Error/debug
output contains status rather than token or descriptor data.

Eight local unit cases cover these lifetimes, eight late context names, ten
original input/kernel/policy faults, four lock/directory replacement cases,
post-binding trust/authority drift and independent concurrent owners. A seeded
five-position test mutates an actual modeled source immediately before each
check and requires the corresponding first error with no later checkpoint.
An independently opened witness of the original lock still observes exclusion
even after the public lock name changes. These are private nonroot filesystem
and synthetic issuer fixtures with actual local kernel packet/context checks,
not a verified protected GitHub job or positive production enrollment.

The first API-proof run illustrates another false-green risk: cargo reports nine
compile-fail examples passed even though Rust1.94 internally panics while
diagnosing a nonexistent extractor. That run is retained but not accepted as
proof. The example instead attempts the private original field and gets E0616;
all nine examples then produce ordinary expected errors on Rust1.94 and Rust1.88,
without a compiler panic. The runtime API and compiler versions are unchanged.

Strict Linux MSRV lint separately rejects the constructor's at-least176-byte Err
variant. Boxing only the original fence inside the failure value preserves its
ownership and drop/unlock behavior without suppressing the lint. This bounded
failure-path representation is not an allocations/op or throughput result.

This addition writes nothing: it consumes no nonce, acknowledges no attempt and
publishes no state or durable refusal. Its sequential bracket is neither an
atomic snapshot nor a new whole-operation deadline. Existing publication/start
APIs cannot consume it. The next writable join must retain this same fence and
original authority through durable attempt acknowledgement, then exercise every
new SIGKILL/lost-ack boundary. Production runtime/job/issuer selection, authenticated
original-start and P2/P3 all-writer cessation remain before completion and server
admission. The [evidence packet](../testing/performance/0.74/local-runs/controller-fence-preflight-20261015/README.md)
keeps the missing-method negative and separates modeled checks from those still
unopened execution rights. No RESP/native performance claim follows.

### Retaining the original controller request before execution

The next P2 gap is the transition from a healthy read-only preflight to a
durable attempt. The existing P1 attempt API takes a different signed intent.
Passing an extracted request into that API, or opening another authority owner,
would discard the original controller/OIDC/kernel binding. The new opt-in path
instead consumes the original common-fence preflight by value and keeps its
same exclusive controller chain and uninterrupted lock.

Only the original Reserve request at expected revision zero can enter. The
canonical record binds its request/hash, token digest, builder-policy/receipt
digests, original public configuration digests, and active original authority
revision, policy digest and epoch. It contains no bearer token or private key.
The token and signatures remain verified through the runtime owners; the stored
record itself is unsigned. Its standalone validator establishes byte consistency,
not independent origin authenticity or recovered permission.

The private descriptor-relative attempt engine now selects between two fixed
filenames. Existing signed-intent attempts keep their old name and bytes; the
controller request uses `diagnostic-controller-attempt-074.json` and a separately
registered v1 schema. The file is created exclusively in the original empty
leaf, retained by descriptor, sealed read-only and synced with its directory.
All eight creation/write/seal/sync boundaries recheck the same original
controller and captured binding. A final storage/controller bracket must pass
before returning runtime acknowledgement.

Any first error retains partial bytes and the original lock while permanently
refusing both original chains. Even after healthy drop, the retained leaf blocks
a fresh empty-fence reservation. This is conservative host-leaf exclusion, not
a global nonce ledger: before the first create there is no durable admission,
and whole-host rollback is not prevented. Lost acknowledgement never becomes
permission to retry from a readable file. The old lifecycle publisher explicitly
rejects controller-attempt documents, preventing a format-compatible-looking
storage object from becoming a signed-intent execution path.

The test-first positive case fails on the missing retention method. Logical
fault tests then cut all eight write boundaries with seed `0x0740c094`, while a
second matrix changes the original trust source at each boundary. Other cases
check strict reader mutations, signed non-Reserve rejection, original-object
and namespace drift, retained-byte integrity, sticky no-I/O refusal, lock
lifetime and independent concurrent chains. One development test incorrectly
used campaign acquisition to demonstrate lock release; that operation itself
created a campaign marker. Replacing it with the existing lock-only operation
tests release without changing the admission context. The negative run remains
evidence of the fixture correction, not a production lock defect.

These callbacks establish local interruption behavior, not actual process death.
At this stage the joined original-kernel SIGKILL/lost-ack matrix is still required;
older storage-only crash tests cannot substitute for it. The runtime result has
no start or lifecycle conversion. Actual production policy/epoch and issuer/job
enrollment, authenticated original-start, watchdog and all-writer cessation
remain before completion, later cells and server admission. No allocation or
throughput improvement is inferred from this control-plane implementation.

The [local evidence packet](../testing/performance/0.74/local-runs/controller-attempt-20261015/README.md)
retains the development negatives and the final ten-case repetitions, MSRV/API
checks and legacy process-loss regression. The new composition's real process
loss is not a result inferred from those tests.

### Killing the original controller attempt owner

The next experiment tests a narrower question than server readiness: does the
actual controller/OIDC/kernel/authority/common-fence composition preserve an
incomplete or acknowledged request when its original process dies? Logical
callbacks cannot answer that question because Rust destructors still run after
a returned error. SIGKILL bypasses those destructors and releases ownership
through kernel process teardown.

A separate ignored nonroot Linux matrix uses seed `0x0740c095` to permute all
eight existing write checkpoints and AfterReturn. The child constructs the same
original chain with synthetic trust fixtures and an actual local SOCK_SEQPACKET
packet. Its sender is the child itself, not an external production controller.
At each selected checkpoint it flushes a bounded readiness record and waits.
The parent pins its exact child by pidfd, proves the common lock is busy, sends
SIGKILL through that descriptor and requires signal 9 on wait. An uninterrupted
AfterReturn control distinguishes process loss from normal drop.

The first development run fails before any checkpoint: putting the authority
fixture under the nested child TMPDIR violates the existing strict /tmp leaf
rule. That is an invalid test origin, not evidence that the runtime should
accept more paths. A private helper now uses a separate parent-owned authority
leaf of the already permitted form; the opener and its rejection rule remain
unchanged. All other temporary fixture roots stay below the parent's private
TMPDIR. The failed run is retained alongside the corrected run.

The parent inspects exact leaf inventory, ownership, inode, mode, size, bytes and
mtime/ctime before and after termination. Complete documents also match the
original configuration digests, builder policy/receipt and active authority
revision/epoch/policy. Created leaves an empty file; PrefixWritten leaves
incomplete JSON; complete bytes appear before sealing and become mode 0400 at
Sealed. No lifecycle record, campaign marker or durable refusal appears.

Two separately started readers then attempt only the existing empty-fence
admission. BeforeCreate is still empty and unadmitted. Every retained partial
or complete entry returns Conflict without rewriting or deleting anything.
AfterReturn is especially important: even a complete synced document cannot
reconstruct the runtime acknowledgement lost with its owner. The same original
lock is available after termination, but that availability does not erase the
retained refusal context or authorize another attempt.

This is process-loss evidence on a live local filesystem, not a power-loss,
reboot, rollback or global nonce-consumption proof. The test adds no runtime API,
format or execution route. Owned lifecycle integration, independently selected
production policy/epoch and job trust, authenticated original-start, autonomous
watchdog and cessation of every writer remain separate prerequisites. No
allocation, goodput or latency result follows from this safety experiment.

The regression run also retains an unexplained Kernel(Invalid) while constructing
the original build-bound guard, before the attempt's first write. Its isolated
same-source rerun and subsequent full original-input suite pass. The available
timesync journal does not establish which predicate refused or why. This remains
an attribution task before integrated production admission; later green matrices
must not be used to erase it or justify relaxing kernel/time checks.

The [separate process packet](../testing/performance/0.74/local-runs/controller-attempt-crash-20261016/README.md)
pins final source `11a7e5cf`. Three Rust1.94 repetitions and one Rust1.88 repetition
each pass nine actual SIGKILL cases, one uninterrupted control and twenty fresh
inspections. The original-input regression subsequently passes 69 tests and the
fence group passes eleven, while all development failures remain retained.
These are scoped safety results, not permission to start a server or a numerical
performance claim.

### Locating the first kernel refusal without changing admission

A generic Kernel(Invalid) can come from very different checks: procfs or nsfs
origin, retained object identity, credential projection, clock ordering or the
exclusive packet-hold boundary. The earlier original-controller regression
failed during reviewed build binding before the attempted write. Its subsequent
successes and nearby timesync entries could not identify which check refused.
Changing timing policy on that evidence would substitute a guess for attribution.

The next addition is therefore a private test-only classifier. A scoped
thread-local frame keeps the first static site, file/line and fixture phase.
Clock-order and hold errors also retain the six existing numeric facts: before
and after boottime, before and after realtime, equal boot and the unchanged
hold limit. No extra clock or proc observation constructs the report. It retains
no request, bearer token, raw credential or configuration body, and creates no
runtime journal or public admission interface.

Nested scopes restore their prior frame, even during unwind. Eight concurrently
active test threads prove frame separation. A caught panic canary checks that
reporting preserves the original panic payload; its deliberately emitted trace
is not a reproduction of the historical fixture failure. Positive checks produce
no event, and an event in an expected negative test describes that test's first
refusal rather than automatically explaining a later assertion.

Instrumentation has its own correctness traps. Constructing an error eagerly
inside Option::ok_or would emit a false diagnostic even when the Option is Some.
Making every constructor lazy avoids that in tests, but strict Clippy correctly
rejects the unnecessary closure when test recording disappears. An explicit
compile-time split now keeps the original ordinary-build ok_or while only tests
perform lazy recording. No lint suppression or guard change is needed.

An independent token comparison expands only the approved macros and compares
both non-test source prefixes with the exact earlier baseline. It does not
discard arbitrary blocks, predicates or observation order. Its first run caught
a closure block introduced by formatting a long diagnostic label; a shorter
label restored the original expression shape. Only question-mark joint spacing
is normalized. Changed deadline and filesystem guards must still differ, and a
parsed trailing test module cannot conceal newly appended ordinary code.

The classification matrix covers reversed clocks, the exclusive 60-second hold,
timespec overflow, namespace type/object, proc-origin and credential-projection
failures. Seeded synthetic faults establish that those categories remain
distinguishable and fail closed. They do not establish which category caused
the old uninstrumented refusal. Bounded original-input repetitions may provide
a new event, but neither a new event nor a clean repetition can retroactively
prove the earlier cause. Original-start, all-writer cessation, production trust
and server admission remain separate requirements; this is not a throughput
or allocations result.

The bounded experiment subsequently completed three original-input repetitions
on Rust 1.94 and one on Rust 1.88, each with 69 passing tests and the existing
manual case ignored. None emitted an unexpected first-refusal event. Both
compiler versions passed the 25-test kernel group. The useful result is a
guard-preserving attribution tool, not a reproduced or repaired historical
failure. The [local evidence packet](../testing/performance/0.74/local-runs/kernel-refusal-attribution-20261016/README.md)
retains development failures alongside final checks; the old cause remains open.

### Keeping collector requirements separate from evidence of original start

Changing output ownership changes what a launch signature must mean. The old
unit projection sends worker output to ordinary append files. In the approved
collector architecture the worker receives only pipe writers, while a root-owned
collector exclusively writes evidence files. Signing the old projection and
silently substituting pipes would leave a gap between reviewed intent and the
actual launch requirements, even if both versions use the same executable.

The [collector intent projection](../testing/performance/0.74/diagnostic-collector-intent-design.md)
therefore has distinct body, envelope, unit and signature-domain identities.
Its unit digest includes typed ordered worker properties and a separate output
contract. The fixed command, arguments, environment and runtime/isolation limits
are retained. Worker regular-file output and writable output paths are removed;
the worker uses the root working directory and a zero regular-file size limit.
Semantic stdout/stderr slots name the future owned pipe writers without treating
an arbitrary FD number as a stable identity.

The output contract commits exclusive root-private collector sinks, bounded
stream bytes and the finalization obligations: authenticated original start,
revocation and emptiness of the original cgroup, closure of every creator/manager
writer copy, both actual read EOFs, byte/hash reconciliation, sync, read-only
conversion and explicit evidence-writer close. A signature authenticates those
requirements; it does not establish that any operation has occurred. The returned
projection is deliberately not a transient-unit specification or start capability.

Tests use synthetic keys to check old/new refusal in both directions, domain
substitution even under the same key, re-signed wrong output digests, every
output requirement's inclusion in the hash and ordered slot/finalization arrays.
The checked guard retains its original worker, full state, peer, configuration
and envelope. Restoring a changed input cannot clear its refusal, and original
worker trust failures survive wrapper destruction. Independent byte verifications
remain possible: replay prevention belongs to retained admission ownership, not
to this signature verifier. The existing v1 verifier and unit builder remain
unchanged. Actual root preparation, original-start authentication and autonomous
all-writer cessation are the next integration boundary, not a result of this
byte-only slice or a numerical throughput/allocations claim.

The [retained local packet](../testing/performance/0.74/local-runs/collector-intent-20261016/README.md)
pins runtime `9e936fcf` and final source `9303bb7e`. Three Rust 1.94 repetitions
and one Rust 1.88 repetition each pass 30 tests, including the 16 legacy cases.
Both compilers pass two existing typed-projection units and five compile-fail
examples; the full static suite passes 170 tests. Eight Linux/Windows
check/strict-Clippy lanes pass, while ship admission remains expected-red.
The confirmed result is separation of signed output requirements, not production
execution, performance improvement or closure of the historical kernel refusal.

### Binding collector requirements to the original kernel and request owners

The next composition exposed an ownership problem: the byte-only collector
guard already borrows a checked worker, while the original controller chain owns
that same policy through its kernel/authority binding. Creating a second checked
worker would replace the original owner rather than preserve it. Supplying peer
and time again would also reintroduce assertions at the very boundary intended
to remove them.

The [original collector preflight](../testing/performance/0.74/diagnostic-original-collector-design.md)
therefore borrows the acknowledged request attempt exclusively. Its private
verification uses the existing kernel worker, configured controller key, issuer/
policy/epoch, original sender credentials and a fresh receiver clock sample.
Supplementary groups and the per-message primary GID form one sorted unique
peer projection; no oversized group set is silently truncated to gain admission.
Existing sender/context checks and the packet's original 60-second hold remain
in place. The public preflight accepts neither peer nor time nor trust inputs.

The body must match the original request, nonce, controller and complete validity
window. State identity must also match, with only initial revision one/cell zero
Reserved projections admitted. The same attempt/fence surrounds verification;
dropping the wrapper does not release that original lock. First failure refuses
all original chains, and restoration cannot cause another named-record read or
a retry. No new durable record, state mutation, output or descriptor is produced.

Tests sign mutations under the same synthetic controller key, so refusal cannot
be credited merely to a bad signature. They vary the request/nonce/controller,
window, state, policy epoch and output digest, including a positive but long-
expired window that must fail against the receiver's clock. Other cases retain
the original lock/bytes, change owned trust or inventory, remove a fixture record
after refusal and verify no reopening, and exercise independent original chains.
The historical source comparisons skip only the exact new module registrations;
all old guards and budget canaries remain checked.

The remaining distinction is substantial: a signed supplied Reserved projection
is not an authenticated durable Reserved/Starting owner. This preflight is not
permission to create root outputs or execute a unit. Joining the original
lifecycle owner and trusted root ancestry remains ahead of exclusive creation,
original-start and autonomous all-writer cessation. The standalone byte verifier
keeps its existing interface and scope; neither path establishes production
enrollment or a numerical performance result.

The [retained packet](../testing/performance/0.74/local-runs/original-collector-20261016/README.md)
pins final source `f76cd63e` and the original-controller composition implemented
in `facfe6c2`. The full original-input group passes 76 tests on both Rust 1.94
and Rust 1.88, with its existing manual process matrix ignored there. Each
compiler separately passes the 30-test byte regression, 26-test kernel group
and four compile-fail examples. All 171 static contracts and eight affected
build/lint lanes pass. These local kernel observations use synthetic trust;
they close the read-only join, not durable lifecycle or production execution.

### Creating the initial reservation state from the original kernel clock

Authenticating a controller's signature does not turn the signed clock fields
into receiver observations. The preceding preflight accepted an entire supplied
Reserved state. Its signature could authenticate that statement, but could not
establish when the host actually observed the reservation. Persisting that state
as kernel-backed lifecycle authority would therefore cross an unproved boundary.

The [observed reservation path](../testing/performance/0.74/diagnostic-original-collector-design.md#observed-initial-state-under-the-same-owner)
consumes the acknowledged original request owner. Its first successful check
samples the same retained receiver origin and creates revision-one, cell-zero
Reserved using the original request identity, observed boot and CLOCK_BOOTTIME.
All three initial clock fields use that one sample. Later observations check
clock order but do not move lease origin, report new controller progress or renew
the original packet hold. The configured authority epoch still supplies authority;
wall time is only part of bounded request freshness.

The controller can obtain a cloned state to sign, but cannot replace the owned
state through that copy. A distinct collector preflight accepts bytes only and
compares their signed state digest to the frozen original. The supplied-state
guard cannot convert into this type. Original worker, authority, kernel packet,
controller and fence remain under the same exclusive ownership, and a first
error permanently refuses that chain without retrying I/O on subsequent calls.

Tests bracket the captured origin with actual local CLOCK_BOOTTIME observations,
verify stable exports and retained request objects, and sign six changed state
digests under the valid synthetic controller key. Other cases cover prior
refusal, missing records, trust/inventory drift and independent owners. This is
functional kernel-context evidence, not an allocation or throughput measurement.
The state is still in memory: a separately versioned durable publication and its
real process-loss matrix must precede root output construction and original start.

The [retained local packet](../testing/performance/0.74/local-runs/observed-reservation-20261016/README.md)
pins runtime `77fdb7b8` and final source `82312a02`. On each of Rust 1.94 and
1.88, the original-input group passes 82 tests, the byte regression 30, the
kernel group 26, and ten compile-fail examples pass. The original-input/kernel
groups each leave their existing manual case ignored. The complete static suite
passes 172 tests and all eight Linux/Windows build/strict-Clippy lanes pass.
The confirmed result is an observed frozen state under the original owner,
not durable lifecycle or production execution.

### Retaining the observed reservation without reopening authority

The next hypothesis was about ownership rather than throughput: could the
receiver-observed initial state become durable while preserving the original
request, worker/authority and common fence? Reopening a second journal or
transferring a collector request into the old regular-file publisher would
discard exactly the provenance the previous checks established. Relaxing the
old one-document inventory would also make unrelated retained context look
eligible for adoption.

The separate [collector Reserved publication](../testing/performance/0.74/diagnostic-collector-reserved-design.md)
therefore consumes the observed owner into a private two-document storage engine.
It preserves the original request descriptor, bytes, stamp and fence, and creates
only a new fixed-name record. That versioned document contains the frozen state,
signed collector envelope and hash of the complete original request-attempt
record, including its policy/epoch and build/configuration bindings. Legacy
one-document checks and publication-transfer predicates stay unchanged.

Exclusive descriptor-relative creation, two bounded writes, sealing and file/
directory sync precede acknowledgement. At every named boundary the same original
live chain checks the request, signature, configured authority and receiver clock.
A failure retains the lock and uncertain bytes; it cannot repair, reopen or retry
the attempt into success. Returning a healthy guard acknowledges this record,
not root output construction, authenticated start or renewed controller progress.

The paired byte inspector has a deliberately narrower job. It rejects schema,
canonicalization, request/window/identity, policy/epoch and state/unit mismatches,
but does not authenticate signatures or kernel observations. A syntactically
valid zero signature can pass it. That canary prevents a readable durable record
from being mistaken for a reconstructed runtime permission after process loss.

The focused local tests pass ten cases, including all nine logical failure
boundaries and twenty record mutations. They cover occupied names, original-file
replacement, ancestry/source drift, sticky refusal, independently owned attempts
and untouched request bytes. Both Rust versions pass fifteen negative API
examples and the thirty-test signed-projection regression; all eight build/lint
lanes and 173 static contracts pass. These are functional publication checks,
not numerical performance evidence or closure of Starting and production launch.

The [retained process packet](../testing/performance/0.74/local-runs/collector-reserved-20261016/README.md)
pins source `6c49e3f9`. Three repetitions each pass ten owned SIGKILL cases,
one uninterrupted control and twenty-two fresh inspections: thirty kills,
three controls and sixty-six inspections in total. The BeforeCreate case now
refuses fresh admission too because the original request already exists.
Complete records after lost acknowledgement remain historical bytes, not
recoverable runtime permissions. Snapshots stay unchanged and the original lock
releases after the owner dies. This establishes local process-loss behavior,
not power-loss/reboot durability, installed-root enrollment or output writer
cessation. Starting still needs a separate consuming protocol and authenticated
controller progress before root output and original-start integration.

Final original-input regressions pass 92 tests on each compiler, with two manual
cases deliberately ignored in that group. Each compiler separately passes the
26-test kernel group, 30-test projection regression and fifteen compile-fail API
examples. These retain the original ownership and refusal boundaries around the
new durable path. The confirmed result is initial retention under the original
authority/fence and conservative process-loss handling; production launch,
performance qualification and the historical Kernel(Invalid) cause remain open.

### Authenticating a new heartbeat without renewing admission

The next hypothesis separates retained state from controller presence. Re-reading
an intact Reserved record does not show that the controller has sent anything
since Reserve. The [authenticated heartbeat preflight](../testing/performance/0.74/diagnostic-collector-progress-design.md)
therefore consumes the original reservation together with one newly received
kernel packet, keeping the same authority owner, request and common fence.

Receiver identity is a private object relationship, not an equality of host
strings: the two packets must originate from the same retained receiver. The
new receive point must follow the owner's previous observation. Actual sender
credentials and current kernel time enter the existing signature/principal
verifier with the original configured controller policy. The authenticated
request must be Heartbeat for the same controller, identity, source, preset and
revision, with a request id and nonce different from Reserve. A correctly signed
request for another controller run is still refused.

Freshness stays anchored to the new packet's receive BOOTTIME and expires
exclusively after ten seconds. Neither repeated checks nor a valid signature
change that point. The original admission packet's 60-second hold, signed
request/collector windows and OIDC/build/worker checks also remain in force.
This keeps a follow-up from silently extending the authorization that admitted
the original attempt. The result owns both packets' provenance through the
original chain, but provides no state mutation, further-heartbeat method or
Starting capability. Its failure retains the same fence and cannot retry.

The local checks distinguish real kernel messages and file/lock ownership from
pure exact-boundary tests. They also check a real packet's expiration without
renewing receive time. An early composition test required another successful
full revalidation round-trip and failed with Kernel(Invalid). That development
capture is retained rather than relabeled as success; no deadline was enlarged.
The final ownership test checks successful construction and then an explicit
retained-file fault, while the independent expiry test checks the attributed
progress-age refusal. These are different predicates, not a throughput result
or an explanation of the earlier historical uninstrumented refusal.

The confirmed boundary is authentication and bounded freshness of one message
under still-valid original admission. It does not prove uninterrupted presence
since Reserve, recover acknowledgement or revive an expired execution lease.
Receive time is observed at dequeue, not at enqueue or signature generation;
a previously signed queued message is not proof that its controller observed
the durable Reserved result. Execution authority still needs a causal challenge
and replay/commit policy rather than that inference.
The next consuming Starting protocol must commit its lifecycle/replay/deadline
history and signed state before root output or authenticated original start.

On the final implementation, the default-parallel original-input repeat passes
95 tests on Rust 1.94, with two existing manual cases ignored. Rust 1.88 passes
the six-test focused preflight group. Both compilers separately pass 29 kernel
tests, 30 signed-projection regressions and 21 compile-fail examples. Eight
Linux/Windows check/strict-Clippy lanes and 174 static contracts pass.

The first full parallel attempt remains an explicit negative: 93 passes and
two Authentication(Time) failures, one in the old Reserved positive and one
in the new composition. Whole-workspace formatting and read-only gates overlapped
that attempt; the successful repeat used the same code, command and ordinary
parallelism without those tasks. This is not a causal experiment proving that
overlap caused the failures, nor a waiver for shared-load robustness. Original
authorization windows and ten-second freshness were not extended. The local
contract stays non-promotable and release admission remains closed.

### A post-ack challenge is stronger than a newly dequeued heartbeat

The next hypothesis addresses the queue boundary explicitly. A controller can
sign and enqueue a heartbeat before Reserved exists, yet the receiver can dequeue
it afterward. Authentication and receive order alone therefore cannot establish
a response to the acknowledged reservation.

The [causal challenge preflight](../testing/performance/0.74/diagnostic-collector-causal-design.md)
consumes the original acknowledged owner, revalidates it, generates 32 bytes with
the system CSPRNG, then samples issuance time from the original kernel context.
Its bounded canonical document commits the original request, retained Reserved
bytes, frozen state, identity and issuance time. An unchanged signed Heartbeat v1
must echo the domain-separated digest as its nonce. A different authenticated
nonce refuses even if its message was dequeued after challenge issuance. The
challenge is public data, not a secret or bearer execution permission.

The consuming result retains the same authority, storage, fence and original
progress guard. It accepts only one packet and permits no replacement, renewal,
retry or owner recovery from copied bytes. Its exclusive ten-second deadline is
anchored to issuance, not signing or dequeue. Original admission holds and signed
windows still apply. The byte getter may return an expired document; exporting
data is deliberately not a claim that authority remains fresh.

This confirms a response to a post-ack commitment, not that the controller read
every Reserved byte or remained continuously present. It adds no durable state,
Starting transition, output, execution lease or production enrollment. A later
lifecycle handoff must explicitly define which admission predicates discharge
and which configuration, epoch, context, progress and deadline obligations remain
live. Silently omitting old checks or widening their windows is not that handoff.

The new live composition checks an actual response, a genuinely prequeued signed
message, explicit entropy failure and real challenge expiration. Pure checks
separately exercise exact deadline arithmetic, strict canonical/schema bounds
and 69 seeded commitment mutations (0x0740c09d). Seven new negative API examples
prevent cloning, deserialization, double consumption and start/replacement calls.

The initial expanded ordinary regression is an explicit negative: 96 passes,
four Authentication(Time) failures and two existing manual ignores. It failed
without concurrent builds or whole-workspace formatting. Under the separately
committed [tiering amendment](../testing/performance/0.74/diagnostic-collector-causal-tiering-20261016.md),
only the new costly four-fixture composition moves to a registered manual gate.
Its body remains byte-identical; the retained pre-tier source has its own SHA256.
The 900-second harness limit is not a change to any authentication deadline.

Crucially, this tier separation did not fix the ordinary regression: its next
default-parallel run has 98 passes, one Authentication(Time) failure in the old
heartbeat composition and three manual ignores. All old tests remain enabled,
with no serialization or --skip. An isolated new proof cannot erase that failure.
The validation-cost/fixed-window interaction remains open; neither its complete
cause nor arbitrary shared-load robustness is established. These failures also
do not explain the older uninstrumented Kernel(Invalid) capture.

The [local causal packet](../testing/performance/0.74/local-runs/collector-causal-20261016/README.md)
retains development failures, both parallel negatives and the separately named
final checks. It is unsigned consistency evidence, not a clean-build, binary or
trusted execution attestation. No performance gain or server admission follows
from this security boundary; product qualification remains a separate gate.

The four-case manual proof passes on Rust 1.94.0 and 1.88.0, as do four ordinary
focused causal checks on each compiler. Each separately passes 29 kernel tests,
30 projection tests and 28 compile-fail examples. All eight Linux/Windows
check/strict-Clippy lanes and 175 static contracts pass. These specific greens
do not turn the retained default-parallel regression red into success. Before
Starting handoff, attribute and address the validation-cost/window blocker
without changing policy deadlines or weakening original ownership checks.

### Attributing repeated validation before changing its proof boundary

The next [local attribution](../testing/performance/0.74/diagnostic-validation-cost-design.md)
keeps production guard bodies unchanged. Private test-only frames count request
authentication, request signature work and full worker-policy document checks
inside six original-owner stages. Separate first-Time facts retain only the
bounded operation and the issued, expiry and now values already used by the
existing verifier. Diagnostic Instant timers never become authorization clocks.
Nesting, unwind, thread isolation and actual signed-window/error precedence have
their own tests; copied reports carry no runtime authority.

The important repeated work is not limited to the signed request checked at the
top of the chain. Two independently started current-toolchain probes and one
MSRV probe produce identical call counts:

| Local stage | Request authentications | Full worker-policy verifications |
| --- | ---: | ---: |
| Reserved publication | 272 | 2696 |
| Heartbeat binding | 66 | 652 |

On the two current debug-fixture runs, publication takes 24.01/24.09 seconds,
including 18.08/18.19 seconds inside the worker verifier. Binding takes 5.81/5.92
seconds, including 4.40/4.47 seconds there. MSRV observes the same shape. These
are local elapsed-time components, not production CPU/op or a throughput gain.
Request signature duration is already included in request authentication; adding
them would double-count. The worker timer covers parsing, canonicalization,
signature and context comparison together, not cryptography alone. Scheduler
delay and other unmeasured work remain separate from any filesystem hypothesis.

This identifies an owner for a subsequent isolated proof-reuse proposal. It does
not authorize caching live epoch, authority, configuration or kernel observations.
Any reused immutable proof must stay bound to exact original bytes and key/policy
pins while preserving all existing revocation and context brackets. The old path
must remain a differential oracle before any speedup is claimed.

The instrumented default-parallel original-input repeat passes 99 tests, with the
three prior manual cases and the new cost probe explicitly ignored. No Time
failure is reproduced and no build/whole-workspace formatting overlaps that run.
The earlier failures are still unresolved: changing only diagnostics and obtaining
a green repeat is not their fix or a shared-load guarantee.

Broader checks expose another negative worth keeping distinct. The first current
IPC run has 23 passes and one Host(Busy) in old corrupt-ledger recovery. One repeat
of the same three-target command and ordinary parallelism passes projection,
IPC and worker-policy groups (30/24/15); MSRV passes them too. No old test body,
retry policy or locking guard changed. That occurrence's lock-lifetime/concurrency
cause remains unproved. Six trace tests pass on four platform/compiler lanes,
both Linux compilers pass 29 kernel and 28 negative API checks, eight build/lint
lanes and 176 static contracts pass. The
[retained packet](../testing/performance/0.74/local-runs/validation-cost-20261016/README.md)
keeps the failed IPC attempt alongside the successes. This is a narrower,
reproducible attribution step, not Starting or permission to launch on a server.

The [local heartbeat packet](../testing/performance/0.74/local-runs/collector-progress-20261016/README.md)
pins implementation 0ec52ddc and retains 34 command transcripts with all development
and parallel negatives. Its verifier checks source/frozen inputs and artifact
hashes without treating unsigned logs as binary or execution attestation.

### A failed mechanism expectation stops the optimization screen

The first immutable worker-proof candidate kept the full constructor and every
live authority, epoch, source and kernel check. Inside the same checked owner,
revalidation compared the original trust and context, hashed the signed envelope,
then hashed the retained canonical projection against the original body pin.
Under the existing SHA256 integrity assumption this could reuse the constructor's
signature proof without caching current authority. No field or retained buffer
was added. The exact old method was retained as a private test-only oracle.

The focused tests agree with that oracle across 128 seeded input/trust/context
cases, sixteen top-level projection mutations and four independent concurrent
owners. A single-owner mechanism test first observes ten full verifications,
then one after the change. All four platform/compiler lanes pass those four
tests, and 177 static contracts constrain the exact substitution and unchanged
live guards. Those results establish a local proof boundary, not a timing gain.

The [preregistered screen](../testing/performance/0.74/diagnostic-worker-proof-reuse-plan-20261016.md)
requires five alternating pairs and an upper 95% bootstrap ratio bound of 0.80
for both publication and binding. It does not complete. The first baseline
passes, but the first candidate fails the exact Setup counter: two full
verifications instead of the predicted one. Inspection of the unchanged fixture
explains the extra constructor: it verifies a policy to initialize modeled
authority, then creates the separate original kernel-bound policy owner.
The single-owner mechanism test did not model that second construction site.

The response is to stop, preserve the failure and revert the isolated candidate,
not silently change the expectation and continue the same experiment. No later
pair is run, no confidence interval is calculated and the failed invocation's
shorter elapsed time is not a speedup. The hypothesis remains unmeasured rather
than disproved; a corrected proposal must fix its construction-site accounting
before collecting a new screen, with the same practical floor and signed windows.
The [negative packet](../testing/performance/0.74/local-runs/worker-proof-reuse-20261016/README.md)
retains both binary hashes, the candidate patch, development reds and its rollback.

Separately, the candidate's current IPC regression repeats the old Host(Busy):
23 passes and one failure, without a retry. MSRV IPC passes all 24, and both
compilers pass the 99-test original-input group, 29 kernel tests, four-case manual
causal proof and 28 compile-fail examples. These greens neither explain the IPC
lock lifetime nor close the historical Time/Kernel(Invalid) causes. Product
throughput, Starting and server admission remain outside this local tooling step.

### Reusing immutable proof without caching current authority

The separately [preregistered second screen](../testing/performance/0.74/diagnostic-worker-proof-reuse-v2-plan-20261016.md)
accounts for both construction sites before implementation or measurements.
A new two-owner test first observes eighteen full verifications for two
constructors and sixteen revalidations, then requires exactly two. The prior
single-owner test still requires one. This protects the distinction between
reusing an owner's original proof and sharing admission between independent owners.

The resulting change is deliberately inside the immutable document boundary.
Construction still checks the complete signed canonical document. Revalidation
retains exact trust/key/body-pin/epoch and host-context equality, original envelope
SHA256 and sticky refusal. It also hashes the retained canonical projection to
detect internal corruption. Only the repeated document parse and signature proof
are removed. The retained owner gains no field or buffer. Live source files,
authority epochs, kernel/peer observations and signed time windows still execute
through the unchanged enclosing owners; no current revocation result is cached.

Five independently started pairs now complete in B/C, C/B, B/C, C/B, B/C order,
without a failed invocation, retry or sample exclusion. The two binaries keep the
same before/after hashes. Request-authentication and request-signature counts match
at every stage. Full worker-document verifications change from
99/72/908/308/2696/652 to 2/0/0/0/0/0 across the six stages: both constructors
survive, while repeated publication and binding work disappears.

| Local debug stage | Baseline seconds | Candidate seconds | Paired C/B ratio | 95% interval |
| --- | --- | --- | ---: | --- |
| Reserved publication | 23.45–24.02 | 5.31–5.68 | 0.232739 | 0.228305–0.236526 |
| Heartbeat binding | 5.48–5.83 | 1.29–1.40 | 0.236497 | 0.228309–0.243068 |

The geometric paired-ratio estimator and seeded 10000-resample percentile
bootstrap are unchanged; both upper bounds clear the fixed 0.80 rule. These are
elapsed times from local unoptimized Rust 1.94 WSL fixtures, including diagnostic
instrumentation, not production CPU/op or native/RESP throughput. No measured
allocation reduction is inferred from removing parsing, and the separate stage
gains are not added together. The [second packet](../testing/performance/0.74/local-runs/worker-proof-reuse-v2-20261016/README.md)
retains all ten raw stage reports and independently reproduces their statistics;
the first failed screen and rollback remain separate immutable evidence.

All five portable proof tests pass on four platform/compiler lanes, with the
exact old method retained as a private test-only oracle. Eight build/strict-Clippy
lanes and 177 static contracts pass. Current and MSRV each pass 99 ordinary
original-input tests, 29 kernel tests, all 24 IPC tests, the unchanged four-case
causal proof and 28 compile-fail examples. Neither green IPC run retries a failed
invocation in this screen. They do not explain historical Host(Busy), Time or
Kernel(Invalid), or establish arbitrary shared-load safety.

The useful outcome is narrower than release qualification: repeated immutable
verification is a confirmed local tooling owner, and its isolated replacement
clears the preregistered debug floor without dropping live observations. The next
cost investigation must preserve that boundary. Authenticated original-start,
autonomous all-writer cessation, completion/later cells, real trust enrollment
and server rehearsal still need their own implementations and fault evidence.
