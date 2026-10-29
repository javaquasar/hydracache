# Preview attribution: retain source, decline integration

The first fixed local cohort attributes the existing Chitchat S preview. It
finds additional Rust allocation work on accepted and rejected operations. The
candidate remains archived but is **not accepted for dependency integration**.
No replacement algorithm is tried here. Root dependencies, qualification identity,
all live checks and original 300/60/10, 780/4800 contracts remain unchanged. No
server, services, clocks, installation or product workload is used.

## Matched inputs and first results

The [method](../../../../tools/chitchat-preview-cost-074/README.md) is fixed before
build/measurement. Both release-profile Rust 1.94 images use S's same complete
source, lock, Zstd 0.14.3, LRU 0.18.2 and identical diagnostic overlays. C0 changes
only the bound method back to original B; C1 keeps S. C0 retains the known unsafe
underestimate and is an attribution counterfactual, never an acceptable repair.

Each compiler artifact is bound by actual Cargo source/path and executable SHA,
then its whole catalog is listed: original 106 tests plus exactly three controls.
Both images pass the explicit allocator and original 239-case admission/wire
controls before the cohort. Four processes execute once in order C0,C1,C1,C0.
Each checks all 239 original seeded payload/MTU cases, observes allocation twice
per case and times six fixed CPU batches. All admission decisions and exact wire
bytes agree with original B in every process; repeated allocation records are
identical within/across the two observations and two processes per image.

All native commands keep the original 1200-second local envelope, 4 GiB floor,
five-second capacity polling and retirement. Each Job retires cleanly, and full
fixture/overlay/root input/executable/HEAD guards pass. Compilation uses disjoint
content-derived release targets, not shared candidate executable paths. No
refusal is retried and no earlier failure packet is replaced.

## What the preview costs

71/239 cases enter preview and gain Rust allocations: 23 accepted, 48 rejected.
The remaining 168 have identical observed allocation records. The following
numbers are per one bound prediction on the already prepared node writer.
Original C0 has zero alloc/realloc/requested/peak bytes in that region.

| MTU / value bytes | Admission | C1 alloc / realloc | C1 cumulative requested bytes | C1 peak requested live bytes |
| --- | --- | --- | --- | --- |
| 100 / 4096 | reject | 129 / 10 | 29,041 | 15,051 |
| 1000 / 4096 | reject | 6 / 7 | 26,716 | 16,343 |
| 40000 / 39938 | accept | 3 / 6 | 231,995 | 150,944 |
| 65507 / 65447 | accept | 3 / 6 | 308,620 | 202,018 |

The complete-operation region includes node/KV construction, admission, finish,
wire serialization and destruction. For 100/4096 it changes 19→148 allocs and
0→10 reallocs, cumulative requests 70,921→99,962 bytes; its peak remains 66,055
bytes. For 65507/65447 it changes 14→17 allocs, 6→12 reallocs, cumulative requests
579,678→888,298 and peak 213,600→333,270 bytes. A stable whole-operation peak on a
rejected input does not erase temporary allocation churn inside its prediction.

The initial node's three writer vector lengths sum to 38 bytes in these cases.
That is the source-derived payload duplicated by derived Clone, not the large
total cost: preview then serializes 4,111/39,953/65,462 operation bytes and runs
compression/flush, output growth and repeated front drains on its private copy.
The body is not instrumented to count physical copies, so total memcpy/drain
traffic is **not measured**. No throughput or saved-copy claim follows.

Requested live balance ends at zero in every measured region. This checks the
region's successful Rust alloc/realloc/dealloc balance only. It excludes Zstd's
C allocations, allocator caches, RSS and retained process/native memory. It is
not a native leak or retained-memory qualification.

## Timing boundary and mixed observations

The CPU source is current-thread user+kernel GetThreadTimes; elapsed wall uses
Instant, and allocator tracking is disabled during timing. The delegating
allocator still performs TLS checks. These are instrumented image observations.
Microsoft documents [FILETIME amounts](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getthreadtimes)
in 100 ns units, not a guarantee of that resolution. The actual batches are
quantized to 0/15.625/31.25 ms. The predeclared one-ms minimum cannot by itself
make them precise enough. No CPU ratio, zero-CPU claim or non-regression gate is
accepted, and no repeat is run to improve them.

All first absolute CPU and wall samples remain in the packet. At MTU 100 and
4,096 bytes, complete-operation wall time is 4.783/4.853 us for C0 and
110.667/108.384 us for C1. This agrees with rejection doing expensive preview.
Large accepted inputs give a different wall observation: 40000/39938 is
463.922/442.777 us for C0 versus 347.580/337.416 us for C1; 65507/65447 is
511.955/479.780 versus 439.850/416.592 us. Those faster samples are retained,
not discarded. This small instrumented fixed-order cohort does not isolate
code generation, compression state, scheduling or thermal/order effects enough
to claim a CPU/throughput improvement on accepted input. Allocation amplification
on 71 cases is the reproducible attributable result.

Rust's [GlobalAlloc contract](https://doc.rust-lang.org/std/alloc/trait.GlobalAlloc.html)
also allows compiler allocation elimination/merging. Counts apply to these exact
optimized images and fixture matrix, not every compiler or arbitrary state.

## Checks, decision and remaining work

Four Python controls reject missing/foreign native inventories, duplicate or
changed workload/wire and absent/duplicate diagnostic records. Both release
images compile, list all 109 expected tests and pass all-target/all-feature check
and strict Clippy. Two explicit allocator controls, two original operation
controls and twelve cohort controls pass: **16 selected native test executions**.
This is attribution coverage, not a replacement for the original complete source,
package/workspace, GNU or MSRV gates. Owned Rust overlay formatting passes.

The packet retains exact source/lock/overlay/producers, all first streams and
Job receipts, byte-bound catalogs/artifacts and complete inert analysis. The
previous source patch stack remains in place as review material, marked by this
declined integration decision; no adopted product change needs rollback.

The [raw evidence packet](local-runs/chitchat-preview-cost-20261028/index.json)
has 162 logical members and 90 distinct raw objects, with index SHA-256
`51b3a9de6e09754d617e4a036837b5ef1cbcb4dbbfa01c257a32b3252239a8ab`.
Its predecessor index binds the archived B/S source evidence. Neither packet
executes archived code to verify its byte identities.

There are still zero accepted Release 0.74 performance candidates. Do not carry
preview into the product dependency graph merely to make a functional/security
fixture green. Compatible security source/distribution, the independent Rust 1.88
D-Bus repair, actual consumer/GNU/package gates and original execution/recovery
remain open. Native Zstd retained memory and clean CPU attribution are explicitly
unclosed; no new live or measurement cohort is authorized by this report alone.

The later [Zstd security fault gate](zstd-security-fault-result-20261029.md)
adds selected public context/seekable/legacy coverage and exact upstream library
tests on Windows/GNU 1.94/1.88. It does not rerun this cost cohort, change S,
reverse its decline or qualify allocation/CPU/native retained memory. Root
integration and compatible MTU/source distribution remain open.
