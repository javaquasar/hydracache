# Fresh validation leaf allocation and I/O requests

Baseline is `78b7664c`. The preceding round attribution identifies Boot,
Namespace and ActiveAuthority as observation targets, not permission to remove
checks. This local nonroot control preserves all runtime source bodies, live
checks, their frequency and original 300/60/10 deadlines. No server, service,
clock, installation or product workload is in scope.

Measure actual borrowed boot comparison, each of the four namespace openers,
complete Origin revalidation, and authority inspect at one and seven journal
generations. The authority control uses the public isolated `/tmp` fixture
backend and synthetic signed policies. It shares journal inspection with the
fixed backend but omits fixed ancestry and WorkerTrustInput's wrapper; it cannot
be extrapolated into a complete ActiveAuthority cost or enrollment proof.

Use the existing opt-in thread-local allocator counter. Warm each case 32 times,
then record three serial samples of 1024 operations per case on GNU 1.94 and
MSRV 1.88. Record Rust allocations, reallocations and requested bytes only.
These are not retained memory, libc allocations, CPU/op or cache API metrics.
Print/serialize outside capture. No reference runtime candidate is proposed.

Separately snapshot `/proc/thread-self/io` around the same operation batches
with the allocator inactive. Read one retained counter FD before and after;
record empty brackets to expose reader overhead. Preserve raw deltas and report
only read/write request counts and character bytes, not open/stat/close counts,
syscall latency or disk traffic. `strace` and `perf` were not found in PATH;
do not install a tracer or label source-derived counts as observed syscalls.
Keep these I/O observations separate from allocation samples and from timing.

Require complete operation success and exact original kernel-walk counts in a
separate observer pass. Test I/O parser duplicate/missing/overflow handling,
read observation visibility, namespace/boot drift and sticky authority refusal.
Stop a failing compiler cohort, retain its first failure, and do not retry it
into a healthy result. Existing failed live matrices remain closed: this is not
a new admission run or a replacement for time continuity.

Freeze source and command recipes before observations. Run focused Linux tests,
affected-package check and strict clippy, exact predecessor reconstruction,
formatting, documentation, gated registry and local performance contracts.
Retain hashes, receipts, raw logs, negative results and prior sealed inventory
anchors. Commit instrumentation separately from evidence and conclusions;
do not push through the existing red live gate.

## Local outcome

Instrumentation `463d8753`, measured source
`a574a8ae32ff37ff029f6798a7a1efde1cddd909`, adds only an opt-in test module.
All previous runtime bodies and checks reconstruct exactly. Both compiler
lanes pass 47 focused tests, affected-package check and strict clippy; 361
historical assertions and two new source/contract controls pass. Full formatting,
docs, registry and local non-promotable performance checks also pass.

All eight cases complete three allocation and three I/O batches on each compiler.
Every per-case allocation sample is identical across both lanes. Borrowed boot,
all four namespace openers and full Origin revalidation request zero Rust heap
allocations/reallocations. Boot performs one read request per operation, Origin
two, and namespace zero; zero reads does not mean zero open/stat/close calls.
The empty counter bracket contributes one read, retained raw and subtracted only
from request counts. Every measured operation batch has zero write requests.

Fixture authority inspect at one generation requests 68 allocations,
11 reallocations and 3014 bytes per operation, with four read requests.
At seven generations it requests 184 allocations, 59 reallocations and
13292 bytes, with 28 reads. Requested bytes include realloc requests; they are
not retained memory. Fixed ancestry and the full ActiveAuthority wrapper are
not measured. The 1/7 depth comparison shows remaining allocation work in the
shared journal reader, not a proven linear law or timing improvement.

Both images retain their hashes; their separate processes report the same boot
`8bb31f38-cbcc-4081-8cae-e4bc496179fd`. This does not prove clock continuity.
The first report parser refuses the harness-prefixed origin line before writing
a summary. Its original recipe and failure remain retained; the corrected parser
accepts only the exact harness prefix and reprocesses unchanged logs. Neither
measurement is repeated. The Windows formatter capability failure remains
separate from the successful WSL formatting check.

No runtime candidate is adopted. Allocation removal in these successful Boot,
Namespace and Origin paths has no remaining measured Rust requests to target.
The next isolated hypothesis is authority issuer-key formatting and temporary
name/context ownership; attribute each separately before changing the reader.
Do not cache authority observations, skip inventory/metadata checks or extrapolate
these requests into full-frame latency. Existing live clock-order refusals and
native/server admission remain closed; no push or new server run occurs.

Raw observations and verification recipes are in the
[local packet](local-runs/validation-leaf-requests-20261020/README.md).

Evidence commit `9193dd5afd8031fe8499fde880e1de8064ff27a3` retains 53 members
plus inventory, SHA256
`9861517676b8ed06b49ebe9d3ebd3c9d8d4835c74fd81c4ff6703edccd9f9ad3`.
Read-only verification rehashes all seven preceding sealed packets and rejects
nine summary scope mutations while keeping native/server/product flags false.
