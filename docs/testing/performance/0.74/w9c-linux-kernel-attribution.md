# 0.74 W9c dedicated-Linux scheduler/socket attribution

Status: **complete kernel-boundary attribution; no new product candidate admitted**.

This profile closes the remaining gap between application-level Tokio socket polls and Linux
syscalls. It is non-promotable owner evidence, not a release-capacity comparison and not permission
to run the 6-hour or 24-hour qualification phases.

## Capability boundary and method

The self-hosted Linux runner permits child tracing with `strace 6.8` and socket inspection with
`ss`, but denies tracefs/debugfs syscall events and `perf` even for software counters
(`perf_event_paranoid=4`, `unprivileged_bpf_disabled=2`). The profile therefore launches the RESP
profiler as a traced child rather than attaching to a sibling. A hidden gate stops the child after
preload and warmup, writes one unique marker, and begins the measured workload only after the
orchestrator starts `ss` sampling and creates the go marker.

Six fixed cells cover GET and SET at pipeline 1/concurrency 1, pipeline 10/concurrency 1, and
pipeline 10/concurrency 8. Every process runs 40,000 measured operations after 8,000 warmup
operations with payload 64, key space 4,096, seed 7,409, exact response validation, instrumentation
enabled and loopback TCP. Five counterbalanced repeats produce 30 attempts.

`ptrace` is not a neutral scheduler observer. The first retained campaign showed that tracing made
context switches thousands of times more frequent. The corrected contract therefore uses two
independent processes per attempt: an untraced companion supplies measurement-window `getrusage`
and PID-owned `ss` samples; the traced process supplies syscall counts and endpoint ownership.
Application counters and workload digests must be identical across the pair. In total, 60/60
processes and 30/30 pairs were valid.

The first workflow run, `37303375748`, is retained as invalid harness evidence. Its validator
incorrectly required empty stdout even though the profiler intentionally emits the same receipt to
stdout and a durable file. No sample was hidden or reclassified. The amended source instead parses
both receipts and requires exact equality.

The exact contract is
[`w9c-linux-kernel-attribution-contract.toml`](w9c-linux-kernel-attribution-contract.toml); the
identity-bound summary is
[`local-runs/w9c-linux-kernel-attribution-c126283c.json`](local-runs/w9c-linux-kernel-attribution-c126283c.json).

## Results

The table reports medians across five repeats. Context switches and queue highs come from the
untraced companion; syscall counts come from `strace`. Goodput and latency under tracing are omitted
because the median traced/untraced goodput ratio was only 0.0826.

| Cell | client writes/op | server reads/op | server writes/op | epoll calls/op | untraced context switches/op | max send/recv queue |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| GET p1 c1 | 1.0 | 1.000025 | 1.0 | 2.000125 | 0.000350 | 71 / 71 B |
| GET p10 c1 | 0.1 | 0.100025 | 1.0 | 0.200275 | 0.000575 | 590 / 639 B |
| GET p10 c8 | 0.1 | 0.100200 | 1.0 | 0.032500 | 0.006675 | 710 / 710 B |
| SET p1 c1 | 1.0 | 1.000025 | 1.0 | 2.000150 | 0.000350 | 130 / 130 B |
| SET p10 c1 | 0.1 | 0.100025 | 1.0 | 0.200225 | 0.000500 | 1,300 / 40 B |
| SET p10 c8 | 0.1 | 0.100200 | 1.0 | 0.032100 | 0.005525 | 1,300 / 1,300 B |

Pipeline 10 already combines ten client requests into one TCP write, so client writes and server
reads fall to about 0.1 per operation. The response side does not follow: every cell performs
exactly one application write, one explicit flush and one successful Linux write per response.
Across all 30 attempts, every traced input and output byte reconciled exactly with the RESP receipt,
unknown loopback calls were zero, and server write/read EAGAIN, client write EAGAIN, application
pending writes and short writes were all zero.

The only large EAGAIN count was on client reads under `strace`; it reflects the profiler's
nonblocking client waiting for one reply batch and is paired with the severe observer slowdown. It
is not a server backpressure finding. Untraced socket queues remained small: the largest median
high-water in any cell was 1,300 bytes.

Concurrency 8 increases untraced process context switches to roughly 0.0055--0.0067 per operation,
but this process contains both the load generator and server, and tracefs/perf task-wakeup timing is
unavailable. That observation does not isolate a tunable Tokio or kernel scheduler owner. The
straced context-switch values are explicitly unusable for product reasoning: their median ratio to
the untraced companion was about 6,121 times.

## Decision

The kernel boundary confirms the existing W3 owner: deep pipelines still pay one Linux server
write per reply. It does not authorize another copy of the same candidate. The previous bounded
batch reduced socket write polls to 0.1 per operation and materially improved deep-pipeline cells,
but failed frozen pipeline-1 CPU/tail guards; the adaptive retry failed its backpressure semantics
before measurement. W9c adds no new mechanism that avoids those failures.

No Nagle, socket-buffer, flush, scheduler, runtime or other platform tuning candidate is admitted.
There were no server short writes, pending writes, EAGAIN pressure or large queues to support one,
and task-level wakeup ownership remains unavailable. Platform defaults stay unchanged. A future W3
proposal must introduce a semantically different shallow-free batching design and preregister its
backpressure behavior; this syscall result alone cannot reopen the rejected implementations.

The successful workflow is
[GitHub Actions run 37304417778](https://github.com/javaquasar/hydracache/actions/runs/37304417778).
Its 47,968,560-byte artifact retains all compressed per-thread traces, paired receipts, rusage,
socket samples and digests. The packet is explicitly non-promotable and contains no candidate data.
