# Local preview attribution, fixed first cohort

This attributes the already retained S proposal, not a new product hypothesis.
Do not adopt dependencies, change a live validation, run services or use a server.
Keep the original local review envelope: 1200 seconds per owned command, 4 GiB
capacity floor, five-second capacity polling and five-second Job retirement.

Prepare C0 and C1 from the exact retained S fixture. C1 has its original preview;
C0 replaces only the bound method with the original B method, keeping S's Zstd,
LRU, lifetime fixes and every other byte. C0 is a cost counterfactual with the
known bad bound, not an acceptable security fix. Both get identical test-only
overlays. Sources/locks/compiler/overlays are hashed; release targets are disjoint.
Build and bind actual Cargo compiler artifacts, enumerate the complete original
106 tests plus exactly three named attribution controls. Run the allocator control
on both images before the single fixed cohort C0, C1, C1, C0. No retry-to-pass.

Each cohort process first records all original 239 supported operations (same
seed, node, key, payload lengths and MTUs) and exact wire bytes, comparing both
images and the previously retained B observations. Two allocation observations
per case isolate (a) the bound method on a prepared writer and (b) complete node/
KV admission, finish and wire serialization, including destruction. Inputs are
prepared outside the region. Thread-local System allocator accounting records
successful Rust alloc/realloc calls and requested bytes, peak requested live
bytes and final balance. It excludes C/Zstd allocations, allocator caches, RSS,
OS retained memory and actual physical copies. Zero final balance means no net
Rust allocation in that region, not no process retention or native leaks.

Six predeclared cases get separate CPU measurements with tracking disabled:
(MTU,value bytes,repetitions) = (100,1,1024), (100,4096,64),
(1000,64,1024), (1000,4096,64), (40000,39938,64), (65507,65447,64).
Each has one untimed warm operation and then exactly that many bound and complete
operations. GetThreadTimes measures current-thread user+kernel CPU; Instant
measures wall time. The release profile is optimized, one native test thread.
Report all first samples, order and absolute durations; batches below one ms of
CPU cannot support a ratio. No outlier removal, inferential pass threshold or
whole-system/native API claim. Input generation and semantic diagnostics are
outside timed regions. The fixed-count experiment is not a duration-based W1
qualification workload and does not replace it.

The source-only observer reports vector lengths/threshold before each prediction.
Their sum is the source-derived payload copied by the preview's derived Clone
when its branch executes. It is not a measurement of memcpy traffic or total
copied bytes. The operation's serialized length is additional preview append
input, not an allocator copy estimate. No claim about retained native memory.

Before a measurement, require identical whole fixtures/overlays/root inputs,
the same executable hash/catalog and a passing original 239-case native control.
After each command repeat all guards and require owned Job retirement. Preserve
any refusal before continuing; no replacement of old evidence. New source code
uses unsafe only for the Windows CPU API and a delegating System allocator;
the allocator has an explicit size/reallocation/deallocation control.

The fixed commands are `python -B tools/chitchat-preview-cost-074/run.py prepare`,
then `build-C0`, `build-C1`, `list-C0`, `control-C0`, `delta-C0`, `list-C1`,
`control-C1`, `delta-C1`, followed by `cohort-1` through `cohort-4` exactly once.
Use `check-C0`, `clippy-C0`, `check-C1`, `clippy-C1` for the scoped gates.
All outputs are exclusive under `target/chitchat-preview-cost-20261028`.
The historical prepare requires retained B/S fixtures and receipts; it is not a
standalone package installer. `analyze.py` is the inert first-cohort analyzer;
its original executed predecessor is also retained in the evidence packet.
It refuses a second output directory and executes no native code.

In the first cohort the CPU observations are multiples of 15.625 ms despite
the API's 100 ns unit. The planned one-ms minimum alone does not establish
sufficient resolution. Keep the first zero/coarse values; publish no CPU ratios.
The disabled counter still leaves TLS checks in the delegating allocator, so
CPU/wall data describe instrumented images. They are not clean production CPU.
See [the result](../../docs/testing/performance/0.74/chitchat-preview-cost-result-20261028.md).

Primary API contracts: [GetThreadTimes](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getthreadtimes)
returns current-thread kernel/user amounts in FILETIME units; units are not a
precision guarantee. [GlobalAlloc](https://doc.rust-lang.org/std/alloc/trait.GlobalAlloc.html)
permits allocation elimination/merging: observed calls describe these exact
optimized images, not an invariant of every compiler or arbitrary workload.
