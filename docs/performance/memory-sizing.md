# Memory sizing: measured scope and safe use

This page gives *observed* AX42 reference slopes, not a portable memory reservation formula.
The D0 baseline used source `906aa24cc22ad6b50b824120ed6364208484203a` and workflow
`9c533d1de5a25b83bcb294e5b939064737b7d9fc`; its [complete archived grid](https://github.com/javaquasar/hydracache/blob/main/docs/testing/perf-artifacts/0.71/ax42/d0-baseline/README.md)
contains three independent processes per shape. The D4 candidate has a different SHA and may not
inherit a D0 numerical improvement claim. Keep the workload, allocator, protocol, TLS setting,
instrumentation, host admission and exact phase fixed before comparing numbers.

For the D0 M1 shape grid, the table uses the median steady process RSS at 50,000 and 250,000
keys. The marginal figure is `(median RSS at 250k - median RSS at 50k) / 200k`; it includes the
stored value plus key, metadata, indexes and allocator effects in this scenario.

| Value payload | Median RSS at 50k | Median RSS at 250k | Observed marginal RSS per added key |
| ---: | ---: | ---: | ---: |
| 64 bytes | 30.2 MiB | 113.5 MiB | 437 bytes |
| 256 bytes | 39.4 MiB | 159.3 MiB | 629 bytes |
| 1,024 bytes | 75.9 MiB | 342.4 MiB | 1,397 bytes |
| 4,096 bytes | 222.4 MiB | 1,074.8 MiB | 4,469 bytes |

The D0 M6 TLS-enabled HC/2 connection grid, with no slow consumers, had median steady RSS of
22.4 MiB at 100 connections and 62.0 MiB at 1,000 connections. The *observed* marginal slope
over that interval was 46,171 RSS bytes per additional connection. The 100-connection/100-slow-
consumer cell had 22.4 MiB median RSS; this does not establish that slow clients are free under
other traffic or queue pressure. See the [M6 raw evidence and redaction receipt](https://github.com/javaquasar/hydracache/tree/main/docs/testing/perf-artifacts/0.71/ax42/d0-baseline/m6-connections/raw).

Do not extrapolate either slope to a different value distribution, connection protocol, TLS
profile, service mix, persistence mode, allocator or host. For a deployment estimate, start from
the *same-profile* process floor, add a measured key/value marginal term and a measured
connection term, then reserve separate headroom for allocator fragmentation, transient queues,
file/page cache, kernel slab, background services and recovery. Validate the complete process and
cgroup under the expected peak cardinality and pressure; the conservative W2a retained-byte
estimate is an accounting signal, not a replacement for a process/cgroup memory limit. Do not
turn these D0 observations into a universal per-key budget or a 0.71 candidate RSS claim.

The D4 [evidence index](https://github.com/javaquasar/hydracache/blob/evidence/0.71/ax42/d4/README.md) proves exact-source
qualification of the memory scenarios. It does **not** claim a numerical RSS win: the D3
foundation and safety proposals explicitly forbid that claim, and optional efficiency work is
deferred under the [release policy](https://github.com/javaquasar/hydracache/blob/main/docs/testing/memory/0.71/release-policy.toml). A future profile
or allocator change needs its own preregistered paired campaign before this guidance can be
updated for that candidate.
