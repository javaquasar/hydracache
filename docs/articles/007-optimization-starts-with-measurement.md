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
our branch”: HydraCache may consume only commit
`352e53faa480c9997272b9c70798dd5b5c15d581` from the project-owned `javaquasar/moka` fork. A
branch name is useful for humans but mutable, so the contract rejects it as a dependency identity.
The fork is one focused commit over Moka `v0.12.15`; the receipt also records both source-tree ids,
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

We deliberately did not send an upstream pull request as part of this decision. The receipt says
`not-submitted` and `not-requested`, rather than implying that the Moka maintainers reviewed the API.
An external review has an unbounded schedule, while a fork we own has explicit maintenance and
rollback costs. We accepted those costs: recheck upstream and advisories at least monthly and before
each release candidate, never move the pinned revision in place, and require a new receipt,
lockfile diff, SBOM, and full dependency gate for every revision change. Rollback is one product
commit restoring crates.io Moka 0.12.15 and the previous listener wiring.

D2 therefore opens exactly one door: a later product-integration commit may use the pinned observer
seam on the pre-authorized files and surfaces. It does not open measurement. Candidate runs remain
forbidden until explicit removal, replacement, expiry, capacity eviction, duplicate delivery,
saturation, reconciliation, cancellation, shutdown, reentrancy, panic, compatibility, and rollback
tests pass in HydraCache. Local evidence will still be non-promotable, and the numerical claim still
requires the admitted dedicated-host pairs frozen earlier.

### What changed when the observer entered the product

The product integration landed as commit
`73fc38a131d26e78b246fe93d5edd71d33796bbf`. HydraCache now pins the reviewed Moka revision in both
the manifest and lockfile, and the source policy allowlists only the project fork URL. The revision
itself remains fixed by `Cargo.toml` and `Cargo.lock`; changing it still requires a new dependency
receipt rather than moving a branch or tag.

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
