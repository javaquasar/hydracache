# Private GET owner phase B0: refused before the first benchmark

Preregistration/instrumentation source:
`18ee8cdc0d8e34a69e94e5c7a0afbb8288522013`.
The finite contract planned 400 fresh-process attempts. Four release binaries
were built from this clean SHA with one incremental build cache, then copied
byte-for-byte to separate variant paths. All four hashes, compiler, tool lock,
contract and full schedule were sealed before attempting work. Incremental cache
reuse does not mix variants: distinct executables are captured after each build,
and immutable binary hashes select every scheduled role.

## Outcome and exact boundary

The first timing/A/A/off embedded GET 4 KiB/c1 attempt observed machine-wide
background CPU **57.8125%** in the contract's 500 ms prelaunch sample. The frozen
ceiling is **10%**. The runner refused to create the benchmark process and
stopped the series. It did not change the ceiling, repeat the attempt, continue
other cells, or select a lower-load sample. Exactly one refused attempt survives.

There are **zero benchmark processes started, zero numerical pairs completed and
no CPU/op, goodput, latency or native/allocation comparison**. `exit_code=null`,
empty stdout/stderr, false placement flags and absent raw/READY/GO files match
the prelaunch refusal. A compiled feature's execution is not proven by build
enrollment or a sealed binary hash. This is invalid measurement environment
evidence, not a rejected product hypothesis or a native regression.

Packet: `local-runs/get-owner-b0-18ee8cdc/`, exactly three unmodified files:

| File | Raw SHA-256 |
| --- | --- |
| seal.json | bc9c2ae4ea38801905ee1c96b81df9f8f59ae579a29838c22dcba03f62bcf572 |
| summary.json | 789bd36b7f62c0f220eee304d4d0fe8c1a09d9cefb5c8956b9555be62d716cea |
| attempt-0001/attempt.json | a28d1f46c06569bc936a39bd8b31d90814dd31cf9dd15cea45db79a0b6412e70 |

Offline refusal audit:
`python scripts/perf/performance_get_owner_controls_audit_074.py`.
It verifies the complete finite schedule/contract/lock, first role, command,
over-ceiling witness, absence of execution and exact non-promotable summary;
it never launches a process or retries. Xtask separately pins all three raw
hashes. Full successful-matrix replay remains a different, 400-attempt operation;
the refused packet cannot pass that comparison by omitting 399 attempts.

## What is implemented and checked

The isolated harness, four build combinations, sealed runner and full-matrix
replay are implemented. Focused instrumentation checks are retained in
`local-runs/get-owner-b0-instrumentation-checks-e36d6a52.json`: 20 Rust tool cases
across four builds, 102 targeted xtask tests, 40 Python control/old-replay tests,
three docs-script tests, scoped check/strict lint/format, contract, governance,
documentation sync/links and mdbook. Two new offline refusal audit tests add
tamper/missing/extra-file and no-spawn checks. These are local focused tests,
not full workspace verify, hosted semantics or release qualification.

The measured D3a allocation/requested-live result is unchanged. No product/native/
store/protocol code, root lock, old sealed tool/runner/contract/packet or frozen
0.73 artifacts changed. Qualification manifest raw SHA remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
No rented host, service stop, workflow dispatch or expensive qualification ran.

## Next work and risks

Do not silently rerun this invalidated series. A future measurement needs a
separately preregistered cohort and explicitly verified quiet preconditions, with
the same regression floors and this refused packet retained. The one CPU sample
does not establish a persistent host problem or identify its cause; a later
read-only process snapshot cannot retrospectively attribute that interval.
Other applications and chats are not stopped to manufacture a quiet result.

While numerical timing is unavailable, implement scheduled per-operation latency
and independent HC1/HC2 local controls. Microsecond rounding/closed-loop batch
latency in B0 is not sufficient for full scheduled-tail admission. Matched mTLS/
RESP3, concurrency 32/128, misses/errors/slow readers/size transitions, allocator
active/resident/retained and timed idle/refill remain required. Current-source
hosted feature-on CI also remains unproven. Feature stays default off, accepted
product proposals remain zero, integrated C74 unresolved and ship admission closed.
