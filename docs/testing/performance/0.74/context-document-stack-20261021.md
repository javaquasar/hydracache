# Local context document stack scratch — allocation-only candidate

## Decision and scope

Keep the isolated storage candidate at
`71e784faff8791cce02b1d98d30adff5278cda37`, after preregistration `806ac686`
and the separate baseline `71638d9db2817c1a1d24edddb467e2cbf7a4ec9e`.
The pure allocation hypothesis passes on GNU 1.94 and MSRV 1.88. No preparation
speedup, original-owner success or server admission is established.

The [preregistered experiment](context-document-stack-plan-20261021.md) changes
only the temporary zeroed buffer in worker-context `Document::read_with`.
Lengths below 38 use an invocation-local owned array, exposing only length+1
bytes. Larger lengths retain the original zeroed Vec and arithmetic. In
particular machine/boot reads still receive exactly 34/38 bytes, not the whole
short array. The 38-byte array is not a claim about total enum/stack footprint.

Storage is constructed at the old site, after the first metadata observation.
Both metadata observations, the single read, error propagation, count/newline/
content checks and their short-circuit order remain. No origin, observation
count/order, deadline, read retry, retained successful result or authority is
added. The original windows remain 300/60/10 seconds. This is tooling on the
preparation path, not an RESP/native/embedded product throughput change.

## Reproducible pure comparison

The existing thread-local Rust allocation counter measures nine lengths
`0,1,4,32,33,36,37,38,63`, with 256 constructions each. The candidate control
uses the actual private production storage through a test-only adapter; the
unchanged baseline is rerun in the same pure cohort and matches its sealed
earlier counters. Both compilers report the same values:

| Scope per compiler | Baseline requests / requested bytes | Candidate requests / requested bytes |
| --- | ---: | ---: |
| All 2304 constructions | 2304 / 64768 | 512 / 26368 |
| Production lengths 33 and 37, 512 constructions | 512 / 18432 | 0 / 0 |
| Fallback lengths 38 and 63, 512 constructions | 512 / 26368 | 512 / 26368 |

No reallocation occurs in either arm. Other short sizes also request zero in
the candidate. These are Rust allocator requests, not allocation usable size,
retained memory, RSS, CPU or elapsed time. Local initialization, enum handling
and stack copies still have costs. Old LocalContext spans and call counts do
not turn these counters into saved preparation time or admission headroom.

## Semantic and source controls

Four new pure tests cover the actual helper, independent nested ownership,
zeroing and exact slice size, healthy/boundary/fallback models, explicit first
refusals and a 384-case differential with seed `0x074c5a19`. The differential
compares errors and complete metadata/read event order across malformed bytes,
short/over/empty counts and EINTR/EIO/EBADF/metadata failures. An early metadata
refusal at `usize::MAX` verifies that construction/arithmetic is not moved
ahead of that check. These are synthetic metadata operands, not real file races.

The model preserves the complete production read body; token comparison only
substitutes its two metadata operands and the storage constructor. Source
canaries reconstruct the complete previous worker-context and allocator-test
files, and mutation controls reject changed live predicates. Older guards use
that exact reversal, not a filename exemption. The unchanged baseline file is
also compared with Git. The helper and pure controls have no live capabilities.

## Check outcomes and retained first result

The first cohort passes all pure tests and source canaries. GNU strict clippy
then refuses two manual `is_multiple_of` expressions in the new test source;
MSRV lint passes. The original test source, GNU refusal and all first-cohort
logs are retained. A distinct corrected pure cohort changes only those two
test selectors; it does not change runtime or erase/retry a live refusal.

Corrected checks pass: 25 source canaries on Windows, four candidate + five
unchanged baseline + two allocator-counter tests per Linux compiler, affected
all-target check and strict all-feature clippy on all three lanes, workspace
formatting, doc-check, unchanged 186-gate registration and the local
non-promotable performance contract. No ignored/manual/live fixture executes.
This is a focused step gate, not the full milestone/ship verification.

The [packet](local-runs/context-document-stack-20261021/summary.json) retains
both cohorts, initial and final source, method snapshots and structural logs.
The [reviewer](../../../../tools/context-document-stack-074/review.ps1)
checks its inventory, derives counters, and verifies all 249 members and six
raw seals of earlier packets without changing their frozen source reviewers.
Qualification manifest bytes and prior first failures stay unchanged.

The packet has 54 members plus its inventory; raw inventory SHA-256:
`7a65d50c19622754bda58fd389ef03d3f07254465d96f1e16b586cdec94f71f0`.

## Remaining work

Both first Causal(Time) failures remain at 10042390247/10010835739 ns. There is
no new original-owner/live profile, clock/service change, server/SSH/Actions,
release installation, supervisor operation or product workload. Successful full
material revalidation, cleanup/EOF/fence, authenticated delivery, all-writer
cessation and SIGKILL/lost-ACK/reconnect/server gates remain independent and
closed. A timing comparison needs separate admission and must retain, not
replace, those first refusals.

Next audit the trust-reader component C strings as a separate leaf, starting
with a pure baseline and first-error contract. Do not merge it into this
candidate or assume old source-pinned attribution measures current code.
