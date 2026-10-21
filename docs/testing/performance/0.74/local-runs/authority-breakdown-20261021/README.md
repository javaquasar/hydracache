# Remaining authority requests by actual phase

This is attribution only. After exact removal of test hooks, runtime bytes
remain identical to `ad5d8ad07707cbe5f76d9f01b6d05cf41129adb6`. There is no
runtime candidate, allocation-reduction claim or new admission in this packet.
All live checks and original 300/60/10 windows remain.

## Source and controls

The [plan](../../authority-breakdown-plan-20261021.md) is preregistered in
`60b720ca`; non-overwrite recipes are frozen in `ebd889ef`. Exact capture source
is `20e6fc1217420d02dee91d2ac7ec9bae766a3273`. New controls are under
`tools/long-run-supervisor-074/tests/support/authority_breakdown_controls.rs`
and `authority_breakdown_alloc.rs`. The runtime fixture and allocator files
only gain test hooks. The gated-test registry, local contract, new xtask canary
and its chained exact hook adapter cover instrumentation, not a product change.

GNU 1.94 and MSRV 1.88 each use one capture, 32 warmups, three rotated
1024-operation samples and the same synthetic 1/7-generation policy history.
Entry name/payload sizes and SHA256 hashes bind each fixture. The original
owner, entries, pins and context stay alive. Actual runtime methods own the
full, directory, inventory, record, marker and bind phases.

For allocations, reallocations, requested bytes and read requests, require:

`full = 2 * directory + 2 * inventory + records + markers + bind`

The separately frozen scanner retains the exact live loop and guards through
parity. Its cost plus the prebuilt-set expected comparison must equal actual
inventory. Fd-path formatting is a subset of scan, not another additive phase.
The stale prebuilt set is deliberately not a fresh check: a unit test removes
a marker, retains the old expected match, then requires fresh inventory/full
inspect to refuse and full inspect to latch that refusal without repair.

Allocator capture and retained /proc/thread-self/io observations are separate.
The I/O reader is an exact copy of the unchanged leaf control's bounded parser,
with refusal/observation tests. The original eight-case full-inspect control
must still produce 25/0/1718 at depth one and 101/0/8500 at seven, in successful
Rust allocations/reallocations/requested bytes per operation, with 4/28 reads.

`check.ps1` retains both compiler checks, strict all-feature lint, historical
contracts and repository format/docs/registry/local performance checks.
`profile.ps1` binds six committed source inputs and the test image hash before
and after execution. Failed invocations and existing files cannot be replaced.
`review.ps1 -Mode WriteSummary` reconstructs observations structurally;
`final-check.ps1` verifies frozen workload/qualification/current input blobs.
Seal is write-once, verifies all fourteen preceding packets and their members,
and rejects eight structural JSON and twelve scope mutations. After sealing,
`review.ps1 -Mode Review` is read-only. Do not rerun capture over this packet.

## Local outcome

Both compilers and all three samples agree. Each row is one phase invocation;
records/markers cover every entry at the selected depth, not one file.
Reallocations remain zero throughout.

| Phase | Allocations at 1 / 7 generations | Requested bytes at 1 / 7 | Read requests at 1 / 7 |
| --- | ---: | ---: | ---: |
| Full inspect | 25 / 101 | 1718 / 8500 | 4 / 28 |
| Directory, one call | 2 / 2 | 32 / 32 | 0 / 0 |
| Inventory, one call | 8 / 34 | 462 / 1658 | 0 / 0 |
| Records, all entries | 2 / 14 | 577 / 4433 | 2 / 14 |
| Markers, all entries | 2 / 14 | 89 / 623 | 2 / 14 |
| Final bind/view | 1 / 1 | 64 / 64 | 0 / 0 |
| Scan within inventory | 8 / 34 | 462 / 1658 | 0 / 0 |
| Expected on prebuilt set | 0 / 0 | 0 / 0 | 0 / 0 |
| Fd-path formatting, subset of scan | 1 / 1 | 28 / 28 | 0 / 0 |

After multiplicities, inventory accounts for 16/68 full-path allocations and
924/3316 requested bytes. Records plus markers account for 4/28 allocations
and 666/5056 bytes; two directory checks plus bind account for 5/5 and 128/128.
These sum exactly to full inspection, including its read counts. The unchanged
eight-case control stays equal, including six non-authority cases. Entry sizes
and payload hashes match across compilers. No capture fails or is retried.

Inventory dominates Rust allocation count at these depths; record revalidation
is the largest requested-byte phase at depth seven. This does not locate the
CPU/latency bottleneck. Zero read-counter values do not remove the directory
and metadata checks still present in the unchanged runtime source.

The isolated fd-path String is a bounded next candidate, already measured
within scan. Removing its ownership needs a new preregistered Before/After
control, exact path/error/scan tests and full-path conservation; no saving is
accepted here. Changing the observed container, caching scans or reusing read
buffers would be separate hypotheses, not part of that candidate.

Both lanes pass 199 authority tests, ancillary controls, affected all-target
checks and strict all-feature lint. All 377 historical/current source checks
and repository format/docs/registry/local performance gates pass. Runtime and
qualification inputs remain unchanged; this packet opens no live/server gate.

## Scope

Counts describe successful Rust requests and observed read/write requests,
not CPU/latency, retained memory, copied bytes, all syscalls, fixed production
ancestry, full ActiveAuthority or cache APIs. Separate process/boot observations
do not establish continuity or repair earlier clock-order refusals. Original
delivery, all-writer cessation, lifecycle, SIGKILL/lost-ACK, reconnect and real
trust/enrollment remain independent closed gates. No new server/live matrix,
installation, supervisor, clock/service, product, qualification or push occurs.
