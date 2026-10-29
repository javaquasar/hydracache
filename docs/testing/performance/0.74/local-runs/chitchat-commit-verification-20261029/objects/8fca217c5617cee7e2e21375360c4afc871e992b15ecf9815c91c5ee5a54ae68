# Consuming MTU candidate Q: GNU allocation subgate

Q passes the original 239-case allocation/semantic subgate. It is **not adopted
in Root** and is not release or server admission. This continues the approved MTU
repair after [P's native-C refusal](chitchat-native-cost-result-20261029.md);
B/M/S/D/P sources, positive subgates and negative conclusions remain intact.

## Hypothesis and implementation

P's ambiguous predictor compresses into existing scratch, discards the result,
then appends and compresses again. Q combines budget checking with the actual
append. It serializes once into the existing pending Vec, encodes necessary full
blocks without draining pending until commit, and retains those output bytes on
acceptance. The old-estimate rejection floor remains unchanged. If encoded full
blocks plus a raw tail upper bound fit, the tail waits for normal finish. Only
when that bound fails is the exact tail compressed; an accepted compressed prefix
is retained in the existing scratch, with `Option<usize>` metadata, for finish.
A later append invalidates that tail because its canonical block contents change.

A checkpoint restores output/pending lengths, scratch length and cached bytes on
refusal or unwind. Before overwriting an old cached tail, its meaningful bytes
are backed up in the existing output Vec and removed on commit. No new buffer
pool, shared compression context, allocator or Zstd feature is introduced. Vec
capacity can grow despite logical rollback; this is explicitly **not** a purity,
retained-memory or RSS claim. Cache invalidation can also waste a previously
compressed tail in a longer sequence; the 239-case result does not cover that
resource boundary.

DeltaSerializer consumes `try_append_with_budget`, then applies DeltaBuilder in
the original order. It does not perform a second append. The previous mutable
fits/count helpers are test-only references; the independent immutable S oracle
remains separate. Ordinary public append/finish retain canonical wire layout.
All unsupported-item assertions, MTU/seed/order, source graph and limits remain.
No original-owner/live-check code is changed.

## Baseline before candidate execution

The first runner prepare imported another `prepare` through an inherited Python
search path and refused. The separate r1 context exposed a missing fixed profile
in the child command environment. Exact-path loading and explicit propagation
fix these tooling issues in r2. Both first contexts/streams remain; neither ran
a measurement. This is not a retry of an unchanged measurement candidate.

Unchanged P production code first receives only a test adapter and allocation-free
TLS compression-call observer. Its seven-test result is four passes / three
expected failures: discarded compression, absent cached tail and cheap-branch
panic restoration. In the first compression fixture (threshold 1000, payload
4096 plus node prefix), P executes 10 compression calls where five are needed.
Only after recording this baseline can the runner derive Q. Q passes all seven,
including both prescribed compression fixtures, cached-state restoration after
effectful rejection/panic, continuation, old-floor zero-effects rejection,
192 deterministic stateful steps and eight independent writers. The call test
requires five actual compression calls in that first fixture. This is a focused
call-count result, not a CPU speedup.

The full selected Q run passes **72 tests**, from its 128-entry GNU catalog:
serialize/property/stream/MTU, delta/oversize and state tests, including the seven
new controls. The remaining 56 unique catalog entries are outside this selected
run. Four allocator/semantic controls per image and one observation per process
are accounted separately; they do not constitute full package coverage.

## Matched native allocation cohort

GNU Rust 1.94, release, fixed offline lock/vendor and original seed `0x0741028`.
One predeclared **B/Q/Q/B** cohort; two observations per case/process. B is the
old known-inexact admission attribution control, **never an acceptable product
repair**. Both actual images pass the unchanged allocator, native-C, tagged-Rust
and 239-case semantic controls before observation. The same qualified C link
object, default libc/Zstd allocator/features and source/frame guards apply.
Disjoint P-control/B/Q target directories preserve historical image identities.

| Domain | Result across all 239 cases |
| --- | --- |
| Admission and complete ordered wire bytes | Exact equality |
| Logical Rust allocs/reallocs/requested/live/peak | Exact equality |
| Physical tagged-Rust native fields | Exact equality |
| Native C malloc/calloc/realloc/free and all other recorded fields | Exact equality |
| Within-process and same-image cross-process observations | Exact equality |

There are zero regressions in all twelve predeclared allocation-dominance fields
(allocations, reallocations, requested bytes and peak, across three domains).
All native observer errors, failed/external/live counters remain zero. This
removes P's observed duplicate-compression cost on the original corpus without
moving it into either Rust meter. No CPU ratio, throughput, allocator retention,
RSS/PSS or global stateful dominance is inferred from balanced frees or this table.

## Checks and evidence

Both B/Q GNU 1.94 all-target/all-feature `cargo check` and strict Clippy pass.
Six offline accounting/source controls pass, including independent faults in each
of the twelve dominance fields, malformed/reordered/duplicate semantics,
nonrepeatable observations, seed/scope drift, source capsule and Root pins.
Rustfmt and the existing local, non-promotable performance contract pass.

All four first source contexts, actual link-object bytes, original capsules,
compiler/control streams, image/catalog/flags, raw observations and owned process
receipts are in [the Q packet](local-runs/chitchat-commit-20261029/index.json):
1,143 logical members / 341 objects; SHA-256
`853f8691d7e9e1f0d2c1143908fddaefdb36e0fd24a507cbf7ac4424a3c61a71`.
Compiled test images/build caches are excluded. Current analysis sources and
historical producer bytes are distinguished; archived code is not executed by
byte verification. The predecessor native-P index remains
`3550fa5293b3f61f3b6d06662e3090b17658a42c1ca87c97b32f0c20e6d6c489`.
Final documentation/byte verification is retained separately.

Owned envelopes remain 1200 seconds / native session 1100, four-GiB physical and
native floors, five-second polling/retirement. Product live checks/frequency/order
and 300/60/10, tests 5/10/45, package 780/workspace 4800 are unchanged. No clocks,
services, server, installation, supervisor or product workload are touched.

## Remaining acceptance boundary

Next qualify Q's actual cached multi-append/rejection path for retained capacities
and allocation work, then GNU MSRV 1.88 / Windows compatibility and the separate
CPU/end-to-end and RSS/PSS controls. Preserve first results and decline a failing
candidate without a rescue cohort. Only a fully accepted compatible source can
advance distributable maintained backports/public consumers and Root adoption.

Root lock remains `34d7a21d9833a180460e5a094f9ee7f1effe07dfb43a65d90865798963e0070c`;
deny policy and qualification-manifest digest remain unchanged. Three Root Zstd
findings are still open; unchanged full policy/package/workspace gates are not
rerun to manufacture progress. Original delivery, all-writer retirement, durable
cells 2–4, reconnect, actual enrollment and compatible-host rehearsal remain
independent closure requirements. This subgate does not close Release 0.74.
