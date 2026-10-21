# CPU windows within fresh leaf revalidation

Capture source is `b99c1c759630cf632c0a13a905c3ddf9d822def5`; baseline is
`b6f84539`. Only an opt-in test hook, manual registration and source guards
are added. Runtime bytes, old trace bodies and the preceding CPU probe remain
unchanged after exact hook removal. This accepts no runtime candidate.

The [plan](../../authority-leaf-cpu-plan-20261021.md) selects one of seven
existing owners-observer expressions per run. It does not create another copy
of runtime methods. Each depth one/seven and record/marker cluster compares
actual, uncounted traced, seven selected sites and an empty callback. Thirty-two
warmups precede twenty 1024-operation batches per case. Ten forward and ten
reverse rotations balance every position. Each compiler completes one capture,
then the unchanged owners and eight-case allocation/read controls. No preflight
or measured invocation fails; no observation is retried or overwritten.

## Observed selected windows

These are medians of twenty batch means, in nanoseconds per leaf at depth
seven. They include the selected clock-window overhead, not production call
latency. Outer batch CPU uses a different denominator: a complete record or
marker phase call, visiting seven leaves. All raw values and ranges remain in
[summary.json](summary.json) and the CPU logs.

| Selected expression | GNU record | GNU marker | MSRV record | MSRV marker |
| --- | ---: | ---: | ---: | ---: |
| Held file metadata | 659 | 740 | 621 | 644 |
| open_at | 1157 | 1276 | 1116 | 1088 |
| Before metadata | 682 | 723 | 601 | 662 |
| Parent metadata | 698 | 681 | 630 | 595 |
| Initial Vec capacity | 267 | 273 | 238 | 239 |
| Bounded read_to_end | 811 | 864 | 815 | 819 |
| After metadata | 740 | 743 | 589 | 604 |
| Empty callback | 206 | 215 | 192 | 196 |

Buffer windows span 204.33–511.72 ns on GNU and 193.18–714.06 ns on MSRV
across the depth-seven kinds. Empty ranges are 184.67–550.99 ns and
173.02–779.79 ns, respectively. Their overlap does not establish a separable
initial-allocation CPU bottleneck. It does not mean zero allocator cost:
buffer destruction, comparisons and other bookkeeping lie outside that window.
The empty callback also returns a different value type; it is not a correction
term to subtract from the buffer window.

Open includes CString conversion/destruction and the syscall. Metadata windows
cover the respective metadata call, not all surrounding predicates. Read covers
bounded read_to_end. Their larger medians do not identify which sub-operation
an implementation should remove. All fresh operations and predicates remain.
Do not sum separately observed windows into an exact CPU partition.

| Compiler | Kind at depth seven | Actual median us per phase | Traced median us per phase |
| --- | --- | ---: | ---: |
| GNU 1.94 | Record | 30.39 | 30.60 |
| GNU 1.94 | Marker | 27.47 | 26.89 |
| MSRV 1.88 | Record | 23.99 | 24.15 |
| MSRV 1.88 | Marker | 26.09 | 26.05 |

Actual/traced ranges overlap, but their medians are not forced equal. Selected
observation perturbs the schedule and adds clock/counter work. All outliers
remain, including a depth-one GNU held-metadata batch mean of 10.28 us per
leaf. None is discarded, clamped, assigned a cause or used to tune a threshold.
The allocation-byte hypothesis does not establish initial buffers as the CPU
bottleneck; no buffer optimization or latency gain is accepted here.

## Controls and scope

The clock functions match the preceding probe's bodies exactly. Reported
resolution is one nanosecond, not one-nanosecond accuracy. Sixty-four adjacent
clock pairs span 192–765 ns on GNU and 180–528 ns on MSRV. The installed
counting allocator remains inactive during CPU batches; its TLS bookkeeping,
fixture dispatch/assertions and clock boundaries still contribute. Affinity is
unpinned. [environment.json](environment.json) records the local WSL/kernel,
CPU and compiler context; no tuning is performed.

Both lanes and all three controls retain boot
`2b512bc7-078d-4e6a-a6a7-3565dd41ab6b`. This is not continuity with the
preceding CPU packet's boot or a repair of earlier live/time refusals.
Source guards preserve every preceding runtime byte, observer method and clock
body. Synthetic clocks verify selection, unchanged value/error forwarding,
returned-owner lifetime and refusal on reversal/overflow. Selected callback
counts equal depth times 1024; nested sums never exceed outer CPU.

All 54 owner-decomposition observations per lane match the sealed owners
packet, including actual/traced requests, sites and read counts. The full
control retains 23/99 allocations, zero reallocations, 1662/8444 requested
bytes and 4/28 read requests at depths one/seven. Six other cases, tracked
kernel-call counts, payload/name hashes and policy epochs agree. These are
separate post-CPU controls, not an all-syscall trace of each CPU batch.

Both compilers pass 210 authority tests, ancillary controls, affected all-target
checks and strict all-feature clippy. All 385 source assertions across 22 targets
and repository fmt/docs/registry/local performance checks pass. Twenty manual
authority probes stay ignored in the ordinary selector; capture explicitly
selects only this CPU probe and the two established count controls.

Recipes are frozen `check.ps1`, `profile.ps1`, `environment.ps1`, `review.ps1`
and `final-check.ps1`. The reviewer binds committed inputs, binary/recipe
hashes, raw data and summary facts, rejects eight structural JSON and ten scope
mutations, and verifies all eighteen preceding immutable inventories. After
sealing, use `review.ps1 -Mode Review`; capture refuses existing output files.
A repeated study needs its own packet, never replacement observations here.

One separate candidate may test bounded stack encoding of short read-only
file names in open_at. Directory names, long-name fallback, NUL refusal,
syscall flags and every fresh metadata/read check must remain. Its possible
request reduction is not inferred as CPU benefit from the aggregate open
window; cold/warm controls and fault tests must precede any acceptance.

No server/live matrix, installation, supervisor operation, clock/service
adjustment, product workload, qualification or push occurs. Original
300/60/10 windows, frozen workload/qualification and old live/time refusals
remain. Delivery, all-writer cessation, durable lifecycle, SIGKILL/lost-ACK,
reconnect and real enrollment gates remain independent and closed.
