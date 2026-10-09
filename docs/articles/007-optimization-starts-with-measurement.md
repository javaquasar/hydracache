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
