# Fresh Groups collection avoids vector growth

One isolated leaf candidate reduces allocation requests below ReservedLive by
collecting the bounded `Groups:` integers into invocation-local scratch before
creating the same owned `Vec<u32>` with exact capacity. The parser, retained
Projection type and DiagnosticPeer conversion remain unchanged. This reduces
heap requests in this collection, not the number of fresh validations.

The [preregistered method](projection-groups-plan-20261021.md) and pure baseline
are committed at `55c1983602fb14e1870ea3817cfb31e77c2775d8`; implementation and
candidate controls are `355ab0126b25a8e4a693ee3b70b5a1f944004977`.
The [evidence packet](local-runs/projection-groups-20261021/README.md) retains
both stages, source archives, four additional pure parser controls per compiler,
all check logs and a read-only derived-summary verifier.

The packet contains 52 members plus its inventory. The inventory's raw SHA256 is
`ae7db79c3e579b7ad9f1939d2571aab27452b62f81c89b46b98340381f4b91c6`.

## Preserve the original parser and fresh checks

The previous `Result<Vec<_>, _>` collection grows a vector as the ASCII words
arrive. The new private `Values` collector holds 33 u32 slots and a length, then
copies its populated prefix into the final vector. Thirty-three slots preserve
the original intermediate state: the existing parser takes 33 words and rejects
lengths above 32 only after parsing them. An invalid 33rd number must still fail
as `proc-u32`, and a valid 33rd number must still reach the group-budget refusal.
The helper never truncates an input or retries an error. Its sole production
caller retains the exact `take(33)` bound; source mutations cannot relax it.

Only one collection expression and its module hook change in the old runtime.
Exact substitutions reconstruct the entire prior admission file, including field
selection, duplicate handling, order, credential comparisons, sender/proc reads,
boot/namespace inspections, clocks, refusal latches and absolute deadlines.
The preceding pidfd candidate's source contract also reconstructs this successor
file completely, rather than whitelisting it. No original predicate body is
excluded from the older whole-source guards.

This deliberately does not change Projection into a shared or inline persistent
identity type. Old reference decoders and callers keep their Vec representation.
The scratch adds 33 local u32 slots plus length to the collection operation;
there is no FD, clock, live-success cache or new observation stored in it. Numeric
errors may now avoid partial-vector allocation; error values and ordering remain.
There is no wire, durable-format, product cache or native API change.

## Allocation result for the isolated collection

Both GNU 1.94 and MSRV 1.88 use lengths 0, 1, 4, 5, 16, 32 and 33, 256 calls per
length. The empty case allocates nothing in either arm. Every nonempty case
still allocates once in the candidate; its final vector capacity equals length.
Expected input construction and logging stay outside the allocation capture.

| Rust requests per compiler over 1792 collections | Reference | Candidate |
| --- | ---: | ---: |
| Allocations | 1536 | 1536 |
| Reallocations | 2560 | 0 |
| Requested bytes including reallocations | 237568 | 93184 |

The reference counters match the baseline after candidate wiring. These are
Rust allocator requests, not allocator usable-size, copied-byte instrumentation,
retained memory, libc malloc, RSS, CPU, full validator latency or throughput.
The additional bounded scratch and copy have not been timed; lower heap traffic
does not by itself establish lower ReservedLive cost or ten-second headroom.

## Local controls and unchanged readiness

Three new ordinary units cover 2176 deterministic seeded vectors
(`0x07467201`), independent owned results, every first failing element and visit
order for lengths through 33, and full documents through 34 words. Differential
checks retain error sites/facts and Projection root/node/edge call counts,
including duplicates, invalid and overflowing numbers, ordering, whitespace and
earlier Pid failures. Four older pure controls cover 1024 seeded prefix mutations,
field widths/error order, redacted first-refusal capture and credential mutations.
Ten consuming-join models retain the non-reexecution boundary.

Per compiler, 17 default-feature tests pass, plus five feature-enabled executions
(the three new units and two allocator controls): 22 executions, 19 distinct
tests. The first recipe's `kernel_projection` selector matches zero tests; its
unchanged log remains and is not counted as coverage. Four explicit selectors
provide the additional parser controls. Baseline and candidate Windows source
canaries pass 17 and 19 tests respectively. Affected all-target checks, strict
all-feature clippy, complete workspace formatting, doc-check, the unchanged
186-gate registry and local non-promotable performance contract pass.

No ignored/manual test, original-owner/live profile, changed clock/service,
server, installation, supervisor or product workload occurs. The preceding
material, offline-attribution and pidfd packets retain their exact raw seals and
members. The first Causal(Time) refusals remain at ages 10042390247 ns and
10010835739 ns; windows stay 300/60/10 seconds. The qualification manifest SHA256
is unchanged:
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.

Keep this as an allocation-only candidate. The failed full material gate,
successful cleanup/EOF/fence proof, delivery, all-writer cessation and downstream
SIGKILL/lost-ACK/reconnect and server gates are not closed by a pure collection
test. Further leaves require separate attribution and controls. A future full
timing comparison needs separate admission and must preserve the first refused
cohort rather than retry it into a passing result.
