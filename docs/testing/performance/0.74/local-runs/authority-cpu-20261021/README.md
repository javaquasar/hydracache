# Local CPU attribution of fresh authority phases

Capture source is `bcd8aee4071055b3b79376459f065e12c7ec1b50`, containing
instrumentation commit `f0b31b1369ee4a2aadba319d45b172ee81e7f6bb`. Runtime
bytes restore exactly to baseline `3d227add18d68bea62686ea85f5c225c51ba3f1b`
after removal of the opt-in test hook and manual registration. No runtime
candidate is accepted here.

The [preregistered plan](../../authority-cpu-plan-20261021.md) uses actual
owners-fixture methods, not their traced expression copies. Each compiler has
one successful capture: 32 warmups, fourteen counterbalanced 2048-operation
batches per case at depths one/seven, and a separate unchanged eight-case
allocation/read control. Seven forward and seven reverse rotations place each
case in every position twice. There is no failed or retried measured invocation.

Linux `CLOCK_THREAD_CPUTIME_ID` reports one-nanosecond resolution; this is not
a claim of one-nanosecond accuracy. The 64 adjacent clock pairs span 186–514 ns
on GNU and 191–657 ns on MSRV. Empty batches remain raw, without subtraction;
their per-call ranges are 0.89–41.80 ns on GNU and 1.07–1.83 ns on MSRV.
Negative CPU differences, malformed readings and syscall errors refuse without
fallback. This clock neither adjusts nor validates real time.

## Observed CPU ranges

Numbers are batch CPU divided by 2048, in microseconds per complete fixture
inspection. They are neither wall latency nor API throughput. Compiler lanes
are sequential observations, not a paired compiler comparison.

| Compiler | Generations | Minimum | Median | Maximum |
| --- | ---: | ---: | ---: | ---: |
| GNU 1.94 | 1 | 25.64 | 33.17 | 43.40 |
| GNU 1.94 | 7 | 73.48 | 95.87 | 112.37 |
| MSRV 1.88 | 1 | 25.61 | 32.68 | 40.33 |
| MSRV 1.88 | 7 | 70.14 | 84.50 | 105.67 |

At depth seven, the median of each round's records-plus-markers CPU is
59.19/56.96 us for GNU/MSRV; twice inventory is 25.08/23.50 us. These leaf
phases include fresh open, metadata, bounded reading and comparisons, not
only buffer allocation or copying. Their relative size prioritizes a narrower
leaf revalidation study, not a particular implementation. At depth one,
directory/inventory checks remain important; depth-seven priorities do not
establish a general small-journal bottleneck.

The signed full-minus-components residual ranges from -13.26 to +21.88 us
at depth seven on GNU and -18.08 to +22.61 us on MSRV. Depth-one ranges are
-5.89 to +18.17 us and -6.82 to +9.96 us. Separate phase cache/branch schedules
and local variance prevent an exact CPU partition. Raw negative residuals are
retained, not clamped or converted into a gain. See [summary.json](summary.json)
and the two CPU logs for all phase samples.

## Controls and interpretation

The counting allocator remains installed but inactive. Its TLS bookkeeping,
fixture dispatch/assertions and clock boundaries remain in CPU costs; the
empty case does not measure per-allocation overhead. CPU affinity is unpinned.
[environment.json](environment.json) records WSL2 kernel 6.6.87.2, eight exposed
CPUs, Intel Core Ultra 7 258V and both compiler versions. These are local
diagnostic observations, not production CPU estimates.

Both captures retain boot `196ee350-76a4-4f36-9985-4d9ab77f352b`; this does not
establish continuity with preceding boots or repair old live/time failures.
Payload/name hashes, epochs and all six other control cases agree with the
sealed owners packet. Full inspection remains 23/99 allocations, no
reallocations, 1662/8444 requested bytes and 4/28 read requests per call at
depths one/seven. Tracked kernel-call counts also agree. Raw proc I/O byte
counters remain available but are not an all-syscall or copied-byte metric.

Both compilers pass 208 authority tests, ancillary controls, affected all-target
checks and strict all-feature clippy. All 383 historical source assertions
across 21 targets and repository fmt/docs/registry/local performance checks
pass. Nineteen source-specific manual authority probes remain ignored in the
ordinary test selector; only the two intended manual controls run in capture.

Original GNU clippy refused a range loop in the new rotation-coverage test.
The original source, failed ten-step preflight, original successful MSRV
preflight and repaired eleven-step checks remain. Only the test matrix's
iteration/transposition changes; the measured code is byte-identical across
that repair. No lint is weakened. Effective recipes are `check-repair.ps1`
for compiler lanes, `check.ps1` for xtask/contracts/repository,
`profile-repair.ps1`, `environment.ps1` and `review-repair.ps1`.

Use `review-repair.ps1 -Mode Review` to inspect the sealed packet. Capture
recipes deliberately refuse existing observations; repeating a study needs a
new preregistered packet, never replacement files here. Review reconstructs
JSON facts, checks source/image/recipe hashes, refuses eight structural and
nine scope mutations, and verifies all seventeen prior immutable inventories.

All live predicates, error order, fresh owners and original 300/60/10 windows
remain. No new live matrix, server run, installation, supervisor operation,
clock/service adjustment, product workload, qualification or push occurs.
Delivery, all-writer cessation, lifecycle, SIGKILL/lost-ACK, reconnect and real
enrollment gates remain independent. No native/API gain or admission is claimed.
