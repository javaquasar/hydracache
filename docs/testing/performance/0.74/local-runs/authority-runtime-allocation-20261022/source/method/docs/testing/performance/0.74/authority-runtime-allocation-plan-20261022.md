# Installed authority runtime allocation and retention comparison

Measure the installed owned-storage runtime against the exact pre-install
authority engine. Inherits R-3, R-5, R-7, R-8 and R-10. This is local actual-engine
allocation attribution, not a product throughput or server admission experiment.
Starting checkout is 66d5f371; no production or existing test source is edited.

## Pinned source and common instrumentation

Baseline runtime source is 33c54b0cda8c330b115009cdf092ad002077cedf. Installed runtime
source is 66d5f371 (implementation 37db65f6). Exactly eight supervisor src/tests
files differ; record and verify that complete set. Generate isolated workspace
source overlays from committed inputs, preserving Cargo.toml, Cargo.lock,
dependencies, default features, full runtime bodies and existing test bodies.
Baseline replaces only those eight files with their original Git blobs.

Append one identical test-only hook to each original fixture test file and add
one identical probe/counter. Only the read-only scratch inspector differs: the
baseline has no scratch field. It runs outside counted intervals. No existing
feature allocator or old ignored allocation/model image may execute. System is
wrapped without changing allocation forwarding. Record all input/source hashes,
compiler, release profile, generated-file membership and executable SHA-256.

Build and run counter/semantic controls before freezing the measurement method.
Build receipts distinguish uncommitted harness development from committed runtime
inputs. After method freeze, verify the exact compiled probe/counter/inspector
bytes against the committed method before measuring those fresh images. Keep
all first build/control refusals; never overwrite an attempt or retry a measured
cell into a pass.

## Frozen finite experiment

Use nonroot local WSL, Rust 1.94.0 and MSRV 1.88.0, separate newly owned source
and target directories, offline locked builds, default features, release libtest
images and one exact test per fresh process with one test thread. One local WSL
session owns metadata, both compiler lanes, measurements and postflight.

For each compiler run five fresh baseline/installed pairs in BC, CB, BC, CB, BC
order. Each image uses the same synthetic signed policy fixture seed 74, actual
directory-relative files, canonical record/key/payload corpus, operation order,
iteration count and result assertions. Depths are 1, 7, 31 and 127 (largest active
history below MAX_GENERATIONS=128). Preparation creates history outside capture,
closes its owner, then counts the actual reopen. Its mandatory closing validation
may warm installed scratch and must not be removed.

Capture reopen, 32 full inspect operations, refusal, sticky post-refusal inspect
and owner drop separately. External refusal runs at every depth; validation,
binding, transition and publication-error cleanup also run at depth seven.
There is no added warmup or clock-based estimator. Count a final successful
publication to generation 128 separately at depth 127 before external refusal.
All comparisons require identical corpus digests and logical outcomes.

## Metrics and acceptance boundary

Count successful Rust alloc/alloc_zeroed/realloc requests, requested bytes,
deallocations and released layout bytes. A successful realloc releases its old
layout and requests its new layout for lifetime accounting. Report signed net
live requested bytes and the requested-layout peak delta; neither is malloc
usable size or RSS. Reconcile the owner's measured lifetime to zero after drop.
Independently report original raw capacities, actual scratch slots/capacities,
Leaf/Entry sizes and entry-vector capacity. Refusal must clear every scratch slot.
Read whole-process RSS separately, outside capture, as supplemental observations,
not an allocation/retention substitute or a release memory claim.

Retain every pair and report per-cell ranges and paired differences. Identical
request counts across five pairs support a narrowly scoped mechanism result,
not CPU/latency/goodput, allocator-resident memory or universal superiority.
Variable or negative results remain visible. Do not change thresholds, optimize
the candidate, revert existing approved code or loosen a guard based on data.

Keep all live checks/count/order, 300/60/10 windows, first Causal refusals and raw
qualification digest 11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc.
No live preparation, original start, service/clock change, privileged operation,
server, installation, supervisor action, product workload or SIGKILL campaign.

## Pre-measure provenance correction

The fifth build/control-only attempt exposed Cargo reuse across source roots:
its installed role reported fresh:true with the baseline executable and digest.
Do not signal that attempt or use it as a comparison. Its finite method wait
must refuse before any measurement. Use separate target directories per compiler
and role; require a freshly compiled selected libtest artifact and distinct
baseline/installed paths and SHA-256 before ready. Copying dependency bytes from
that completed build to newly owned targets is allowed only with original/new
hash inventories, no hardlinks/source writes, and exclusion of supervisor package
artifacts/fingerprints. All selected supervisor images must compile afresh.
This correction precedes every measured cell and changes no workload, runtime,
counter, live check, original window or first refusal.
