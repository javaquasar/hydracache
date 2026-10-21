# Authority fd path ownership comparison

The single candidate removes the temporary String used to construct the path
for Directory::inventory's fresh read_dir. It uses the identical decimal
formatter in a 25-byte stack Cursor: 14 prefix bytes and at most 11 i32 bytes.
The fresh scan, observed BTreeSet, UTF-8 conversion, duplicate/budget/parity/
canonical checks, error order and all other runtime bytes remain. No live
observation is cached and original 300/60/10 windows stay unchanged.

## Source and controls

The [plan](../../authority-fd-path-plan-20261021.md) is preregistered in
`3d821c61`; recipes are frozen in `27387553`. Before source is
`3635e560599a4987bcc5f9652ec170d33323e6bd`, with controls and exact source
guards but no runtime change. `89691d5e` retains its observations before
implementation. New control sources are `authority_fd_path_controls.rs`
and `authority_fd_path_alloc.rs` under the supervisor tests/support directory.

Byte/capacity tests cover decimal boundaries, i32 extrema and 4096 descriptors
from seed 0x074fd025. Retained-FD filesystem tests observe fresh mutations;
the existing nine-case differential scan checks malformed/UTF-8, parity,
budget and non-directory errors. Full-inspect drift tests preserve sticky
refusal. Exact adapters restore the old runtime and hooks without changing
historical assertions, refusing unknown or duplicate substitutions while
retaining unrelated drift.

Pure formatting compares owned reference and stack proposal for eight fd
values: 0, 9, 10, 99, 100, 12, i32::MIN and i32::MAX. Each compiler/stage gets
one capture, 32 warmups and three alternating 1024-operation samples. There
is no simulated actual arm: real filesystem tests, exact source guards and
the unchanged eight-case full-inspect control own runtime acceptance.
Negative descriptors are formatter inputs only, not opened or admitted FDs.

At depths one/seven, full inspection performs two inventories. Its reduction
must equal twice the isolated fd-path ownership cost. The six other cases,
read/write observations and tracked kernel counts must stay equal. Reference
and proposal must repeat across samples, compilers and stages. Allocation
capture and retained /proc I/O observations remain separate. The source-specific
old breakdown capture is not replayed against a changed runtime.

Non-overwrite recipes bind committed inputs and test-image hashes before and
after execution. Review reconstructs structural JSON facts, checks paired
conservation and all fifteen preceding immutable packets, then creates a
write-once inventory. Failed measured invocations cannot be replaced.

## Local outcome

After/runtime source is `86dbd967555b0dadd0161d8d6cb151c760222b13`. Both
compilers and all three samples agree. For every descriptor, the reference
uses one allocation and 28 requested bytes per formatting operation; the
proposal uses zero. Reallocations remain zero. Frozen reference/proposal,
full-control workload and qualification input blobs stay identical.

| Full fixture inspect | Allocations Before / After | Requested bytes Before / After | Read requests Before / After |
| --- | ---: | ---: | ---: |
| One generation | 25 / 23 | 1718 / 1662 | 4 / 4 |
| Seven generations | 101 / 99 | 8500 / 8444 | 28 / 28 |

Each full-path reduction is exactly twice the measured path cost: two
allocations and 56 requested bytes. The six non-authority cases and all
tracked kernel counts stay equal. No capture fails or is retried. Both lanes
pass 201 authority tests, ancillary controls, affected all-target checks and
strict all-feature lint; 379 source assertions across 19 targets and repository
format/docs/registry/local performance checks pass. Source canaries restore
the complete previous runtime, not selected guards alone.

The candidate is accepted for local Rust-request reduction. Before/After boots
differ; these independent counter controls prove neither continuity nor clock
repair. Old live failures remain, and earlier host receipts do not transfer
to this source. Remaining observed-name/container and record-read costs need
their own attribution and semantic controls, not cached live observations.

## Scope

This is local nonroot synthetic Rust-request accounting, not CPU/latency,
retained memory, copied bytes, all syscalls, fixed production ancestry,
full ActiveAuthority or cache API throughput. Old clock-order refusals and
independent delivery, writer cessation, lifecycle, SIGKILL/lost-ACK, reconnect
and real enrollment gates remain. No server/live matrix, installation,
supervisor, clock/service, product, qualification or push is authorized here.
