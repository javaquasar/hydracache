# Isolated authority operator correctness results

The separate candidate-only cohort passes test isolation on MSVC Rust 1.94 and
1.88: exactly three tests run and nine are filtered on each compiler. Native
argv and test listing are checked before execution. The prior broad cohort
remains target-isolation REFUSED; this result does not replace its first logs or
promote its unexpected allocation values.

The [preregistered plan](authority-operator-targeted-plan-20261022.md),
[new facts](local-runs/authority-operator-targeted-20261022/facts.json) and
[prior refused cohort](authority-operator-order-20261022.md) keep the two scopes
separate. Runner source is `90f36fb7`; method source is `cd6257f1`. Model/runtime
source has not changed in this step.

## Correctness matrix and command boundary

Each first compiler lane selects candidate_state_priority,
candidate_initial_fault_priority and candidate_publication_and_external_latch.
The typed candidate filter crosses the native boundary as one argument, not a
sequence of character filters. The standalone argv image executes no model; the
following model listing executes no test bodies. Either preflight discrepancy
stops the runner before correctness execution. Receipts retain the ordered
steps, source identity, native vectors and hashes of all first logs.

Each compiler observes 3296 synthetic cases: 80 state/error/round combinations,
3072 initial leaf masks, 96 directory/inventory faults, 24 logical publication
cuts and 24 direct external latches. The reader reconstructs the existing oracle
from sealed history, then compares every candidate row, including diagnostic
trace checksums, logical state and round/event counts. Both compiler lanes agree.
No legacy or owned-allocation row appears in either clean log.

The model validates before evaluating transition/budget, and brackets logical
publication with fresh entry/exit rounds. Failed owners refuse without IO;
earlier IO failures retain priority. Errors evict temporary caches but preserve
original bytes/tokens. Logical after-append failure retains the uncertain state
and permanently refuses subsequent work rather than authorizing replay.

## Evidence limits and next gate

The new reader verifies 22 prior inventory seals and 1282 members, independently
retaining the prior red isolation decision. Mutation controls reject altered
argv, listing/completed membership, missing/duplicate/unknown rows, state/trace/
round drift, receipt hashes/order/source and evidence-output replacement.

This closes only the isolated model correctness gate. There is no new timing,
allocation, retained-memory, RSS or physical-copy measurement. Synthetic owned
tokens and logical append are not real descriptors, canonical signed records,
filesystem write/fsync, authenticated delivery or writer termination. Actual
engine integration and its historical source-guard/snapshot boundary need an
explicit patch audit before runtime mutation. Publication, SIGKILL/lost-ACK,
reconnect and server admission remain separate gates.

Runtime, qualification, original 300/60/10 windows and first Causal refusals
10042390247/10010835739 ns remain unchanged. No WSL, clocks/services, server,
installation, supervisor stop or product workload is involved.

All 21 local verification steps passed, including 97 offline controls, 58 source
guards in 20 binaries, format/check/strict clippy, documentation, 186 registered
gates and the local non-promotable performance contract. Compiler-lane check and
strict clippy also passed. No correctness image was retried.

Inventory SHA-256:
`bd81081959409bf27b188871aa7493f0a42e50cdea658800f45db7c35229d475`.
It covers 81 members, excluding inventory.json. All 82 packet files retained
identical raw working/staged bytes. Evidence commit: `d5f8fe51`.
