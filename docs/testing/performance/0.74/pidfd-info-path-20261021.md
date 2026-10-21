# Stack pathname candidate preserves the fresh pidfd check

One isolated candidate removes heap ownership of the temporary pathname in
`diagnostic_kernel_origin::alive_read`, a repeated fresh check below ReservedLive.
The call still polls the same pidfd, freshly opens the same thread-local procfs
document, invokes its read callback once, checks procfs, decodes the original PID
and rejects malformed/duplicate or mismatching fields in the same order.
No validator, live observation, origin, window or observer schedule changes.

The [method](pidfd-info-path-plan-20261021.md) and baseline are committed at
`1c7e00f327e45be8ec543fcd9b8da21faf481874`; implementation is
`0cb604afdc14adb544ba613fd6d46189401254b4`. The [packet](local-runs/pidfd-info-path-20261021/README.md)
keeps baseline and candidate sources and results separately. The preceding
[offline attribution](held-material-round-attribution-20261021.md) identified
repeated kernel work, not this pathname allocation's exclusive elapsed cost.
This is a measured allocation-only candidate, not a measured timing improvement.

The packet contains 51 members plus its inventory, whose raw SHA256 is
`dc7af6cd78dfc1b69c7443078a8c6e2edc4504e294f74d30df93e085f8471daa`.

## One production substitution

Only the pathname operand of `OpenOptions::open` changes from a formatted String
to `InfoPath::new(pidfd.as_raw_fd()).as_path()`. The private helper copies the
fixed prefix and the existing PidName signed decimal bytes into a bounded owned
stack array. It stores no FD, observation, clock, shared cache or start right.
All signed i32 values, including the minimum, fit prefix length plus eleven
bytes. The Path borrows the helper only for the same synchronous open call.

The complete old origin source is reconstructed by removing exactly one module
hook and reversing this one operand. A whole-file comparison preserves poll,
flags, fresh read, filesystem and PID predicates, error propagation and ordering;
mutation controls reject altered flags, descriptors, read handling and duplicate
hooks. All other old runtime source/test files remain unchanged, apart from
the opt-in allocation-test module hook. In particular ReservedLive orchestration,
Boot/Namespace checks, full frames, authority, absolute deadlines and refusal
latches are untouched. No format/wire artifact, product cache or native API changes.

## Local allocation result

The baseline control runs before wiring the candidate into production. Each
compiler formats nine signed descriptor edges 256 times. The same frozen
reference operation is retained in the candidate control; its source assertion
recognizes the one approved old or new operand, with whole-file source guards
proving the substitution. No open or clock operation occurs inside either
allocation capture; logging and expected-value construction are outside it.

| Pure pathname control per compiler | Reference | Stack candidate |
| --- | ---: | ---: |
| Invocations over nine descriptor edges | 2304 | 2304 |
| Rust allocation requests | 2304 | 0 |
| Rust reallocation requests | 0 | 0 |
| Requested bytes | 115200 | 0 |

GNU 1.94 and MSRV 1.88 produce the same counters. In this isolated operation,
the reference requests one 50-byte allocation per invocation and the candidate
requests none. This is not a claim that Alive or ReservedLive has no allocations:
the fresh open/read and other checks are outside the pure capture. It measures
Rust allocator requests, not libc malloc, exclusive CPU, syscall latency, RSS,
retained memory or throughput. No allocation count is extrapolated into the
old refused cohort or a prediction of sufficient ten-second admission headroom.

## Differential and refusal coverage

Six new ordinary units check signed decimal boundaries, 16384 deterministic
seeded values (`0x074fd174`), independent owned temporaries, fresh self-pidfd
open identity/content/flags and missing-name errno parity. Two consecutive
actual reads retain exactly one Alive and one Document span with the original
edge per invocation. Further controls cover changed/missing/duplicate/invalid
PID documents, mismatching expected PID, EINTR/EIO/EBADF returned once without
retry, and a non-pidfd poll refusal before the read callback.

These own-process units create no campaign, worker, cgroup, systemd unit or
original owner. The low-level leaf does not own sticky admission state; its
ability to read again in an isolated unit does not expose a retry of an admitted
or refused owner. The unchanged consuming-holder models retain that boundary.

Per compiler, 24 default-feature selected units pass (six new pidfd, four document
scratch, four boot comparison, ten consuming join), plus eight feature-enabled
test executions (the same six units and two pure allocation controls). These are
32 selected executions, 26 distinct tests, not original-owner repetitions.
Seventeen source canaries pass; nine affected source canaries are also rechecked
after formatting the new guard. GNU/MSRV and Windows affected-package all-target
checks and strict all-feature clippy pass. Formatting, documentation, the unchanged
186-gate registry and local non-promotable performance contract pass.

The first Windows whole-workspace format check refused OS error 206 (command
length); the Linux check then reported formatting only in the new source guard.
Both logs remain. Formatting that guard and checking the complete workspace again
passes, without runtime changes or replaced profile evidence. No ignored/manual
test, whole original-owner run, clock/service change, supervisor operation,
server, installation, product or expensive qualification workload is performed.

## Readiness remains closed

The original material packet and its first Causal refusals, ages 10042390247 ns
and 10010835739 ns, stay unchanged. The attribution packet remains sealed too.
The candidate does not supply the missing successful revalidation/cleanup footer,
EOF/fence proof, authenticated delivery or full SIGKILL/lost-ACK/reconnect gate.
Qualification remains byte-identical at SHA256
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.

Retain this as an allocation-proven local candidate. Before any future claim
about full preparation cost, separately authorize and preregister a new timing
comparison while keeping the first refused cohort intact; no such run occurs
here. Further fresh-check candidates must stay isolated and preserve exactly
the same call graph and original windows. Server admission remains closed.
