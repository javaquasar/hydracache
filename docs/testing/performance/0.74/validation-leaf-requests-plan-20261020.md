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
