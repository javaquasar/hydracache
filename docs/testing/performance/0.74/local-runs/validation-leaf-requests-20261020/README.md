# Fresh validation leaf request control

This local nonroot packet separates Rust allocation requests from kernel
read/write request counters. It does not time syscalls, measure all syscall
classes, replace the live matrix or admit execution on a server.

The [preregistered plan](../../validation-leaf-requests-plan-20261020.md) fixes
eight cases, 32 warmups, three samples and 1024 operations per sample. Each case
also performs a separate 64-operation kernel-walk count control. Allocator,
kernel-walk and I/O captures never overlap. Serialization follows captures.
Boot equality rereads and validates the document; Namespace opens and checks
the current nsfs object; Origin executes its entire unchanged revalidation.

Authority uses synthetic signed policies and the existing public nonroot
`/tmp/hydracache-authority-*` fixture. The one-generation journal is active;
the seven-generation journal contains three explicit refuse/advance pairs and
ends active. Setup and durable publication occur before any capture. Inspect
uses the actual shared journal reader but not fixed production ancestry or
the surrounding WorkerTrustInput checks. These results cannot be multiplied
into a claimed full ActiveAuthority latency saving.

I/O observations retain a `/proc/thread-self/io` descriptor and bracket each
batch with one positional read before and one after. Empty brackets disclose
the reader's own read request. Raw counters remain in the log; the summary
subtracts only that measured empty read count. `rchar` includes observation
bytes and is not adjusted into workload bytes. Neither character counters
nor request counts represent disk I/O. Open/stat/close, libc allocator calls,
retained memory, CPU and latency remain outside this control.

`check.ps1` records each native command exit immediately and hashes its log.
`profile.ps1` rejects changed tracked code, selects one compiled test image,
binds its before/after hash, source, selected inputs and recipe, and refuses
to overwrite a profile. A failing cohort is never retried. `review.ps1`
recomputes the summary from raw logs, verifies checks and seven preceding
sealed packets, exercises scope-flag mutations, and seals this packet once.
Later verification is read-only; do not reseal an archived packet.

The initial Windows all-workspace formatter hits OS error 206. Its compound
inspection did not capture the formatter's native exit, so it is not a pass;
the limitation is retained separately. Changed files use direct rustfmt,
and the full format check runs through WSL. Missing PATH entries for strace
and perf do not justify installing tools or inventing syscall measurements.

Existing GNU/MSRV live outcomes remain in the preceding round packet. This
control never reruns those matrices, changes a live predicate or deadline,
changes clocks/services, installs a release or contacts the dedicated host.

## Results

Measured source is `a574a8ae32ff37ff029f6798a7a1efde1cddd909`; the opt-in
test change is `463d8753`. Both compiler lanes pass 47 focused tests and
check/strict clippy. All 361 historical assertions plus two new controls pass;
format/docs/registry/local performance checks are green. No full release or
native execution gate is run or promoted.

All three samples on both compilers yield the following per-operation requests.
The six Boot/Namespace/Origin cases are allocation-free under the Rust counter.

| Operation | Allocations | Reallocations | Requested bytes | Reads minus empty bracket |
| --- | ---: | ---: | ---: | ---: |
| Borrowed boot comparison | 0 | 0 | 0 | 1 |
| Each of user mnt pid time namespace | 0 | 0 | 0 | 0 |
| Full Origin revalidation | 0 | 0 | 0 | 2 |
| Fixture authority with 1 generation | 68 | 11 | 3014 | 4 |
| Fixture authority with 7 generations | 184 | 59 | 13292 | 28 |

Every batch has zero observed write requests. Namespace zero reads does not
mean zero syscalls: open/stat/close are not observed here. Both processes report
boot `8bb31f38-cbcc-4081-8cae-e4bc496179fd`; no clock correspondence is proved.

The initial summary parser rejects the exact harness-prefixed ORIGIN line;
`review-refused.ps1` and `report-parser-refusal.json` retain that failure.
The repaired parser accepts only the known harness prefix. It consumes the
original successful logs without rerunning either image. `summary.json`
retains per-sample counts rather than only the table's normalized values.

No runtime candidate or speedup is accepted. Authority issuer-key formatting,
temporary names and inventory ownership are next attribution hypotheses;
required fresh reads and metadata checks are not removal candidates. The
preceding live time refusals remain closed and all commits stay local.
