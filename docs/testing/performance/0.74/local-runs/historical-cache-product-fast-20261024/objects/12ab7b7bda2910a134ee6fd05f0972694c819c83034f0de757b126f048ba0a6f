# Immutable Git-input cache for source assertions only

The first complete source-guard repair takes 546.17 seconds for all 321 selected
tests (315 passes, six failures). Most older assertions repeatedly spawn Git to
retrieve the same pinned historical bodies. This is source-test preparation,
not the runtime ReservedLive/kernel validation cost and not a product workload.
The completed repaired predecessor subsequently passes all 321 in 537.81 seconds;
retain that green membership control separately from the earlier failed repair.

Prepare a separate process-local cache keyed by exact historical revision and
path. Cache only complete successful `git --no-replace-objects show` output,
using the same manifest root and LF normalization as before. One OnceLock per
key deduplicates concurrent lookups without holding the index lock across Git.
Return independent owned strings as before; no partial output or failed loader
may enter the cache. A process-local frozen history snapshot is sufficient:
the reviewed commit objects do not change during a gate. This is not a cache
of refs, current files, expected authority or live observations.
The integrated wrapper accepts only full forty-hex commit object IDs, never
mutable refs or ambiguous abbreviated names. Expand the one existing short
test pin to its exact full object ID without changing its assertion.

Standalone controls precede integration: one complete load for 64 same-key
lookups; separate revision/path/instance identity; sixteen concurrent same-key
callers; independent returned strings; nested different-key loading; failure
without publication. The exact current filesystem reads, current whole-file
acceptance/mutation assertions and original test membership stay active.
Only the historical loader and its duplicate implementation may be composed
with this cache. Do not cache `accepts`, current source reads, policy, epochs,
procfs, boot, namespaces or time, and do not remove or reorder an assertion.

Acceptance requires the standalone controls, exact loader-source delta check,
all current/historical mutation gates, full unchanged 321-test cohort, check,
strict Clippy and formatting. The deterministic loader-call bound is the primary
mechanical result; whole-suite elapsed time is supplemental, not a preregistered
product speedup or evidence of original admission-window fit. Keep every first
compile/lint/cohort refusal, including the duplicate-module Clippy failure in
the preceding source-guard repair. No live, service, clock or server run follows.
