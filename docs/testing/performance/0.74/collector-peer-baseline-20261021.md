# Pure collector peer vector baseline

## Frozen operation

The [preregistered baseline](collector-peer-baseline-plan-20261021.md) freezes
the current `peer` helper in `diagnostic_kernel_collector_inputs.rs` before any
growth candidate: clone supplementary groups into a Vec, push the primary,
sort and deduplicate, returning the same uid and an owned canonical group union.
The production collector file and full live wrapper are unchanged. Only an
opt-in private test hook is added to the allocation-test binary.

Baseline source: `da44628a85779977d5d1cb2f32e324239165eeff`;
preregistration: `80b3df61`.

The baseline measures a complete frozen body whose tokens match production,
not an invocation of a real KernelPacket. It covers one empty case and absent,
present and duplicate-primary shapes at lengths 1/2/3/4/8/16/31/32/33/64. Each
of the 31 cases has 256 constructions. Generic lengths above the upstream
32-group limit are leaf compatibility controls, not admitted kernel inputs.

## Allocation result

GNU 1.94 and MSRV 1.88 report the same main-corpus values:

| Scope per compiler | Constructions | Allocations | Reallocations | Requested bytes |
| --- | ---: | ---: | ---: | ---: |
| Complete frozen operation, all 31 cases | 7936 | 7936 | 7680 | 1798144 |

Every nonempty to_vec allocates exactly the input length, then push grows that
vector once, even when primary is already present or dedup ultimately leaves
one group. Empty input allocates only at push, retaining capacity four. A
separate stage control performs one construction per case and conserves every
whole-operation request: to_vec owns the initial allocation, push owns growth,
and sort/dedup makes no allocator requests on this corpus. These 31 stage
constructions are not pooled into the 7936-construction result.

Examples of observed capacity after push and dedup are four for input lengths
0/1/2, 62 for 31, 64 for 32 and 66 for 33. Capacity does not shrink after dedup.
At 32 distinct supplementary groups with an absent primary, the result has 33
groups, not a truncated list. The allocation counters describe Rust requests,
not usable allocation size, retained memory, RSS, CPU or full preparation wall.
Sort/dedup having zero requests does not mean zero execution cost. The corpus
is not a production frequency distribution or a measured speedup.

## Canonical union and first-error contract

Five new pure tests cover the full-body allocation/stage comparison, 1024
seeded BTreeSet-oracle cases (`0x074c0ee1`), boundary/owned-output cases and a
complete synthetic collector-input wrapper. They retain uid, sorted unique
union, primary membership, integer extremes, input nonmutation and independent
owned output. Explicit 32/33/64/256-length cases establish no generic truncation;
the upstream admission limit itself remains unchanged.

The wrapper has 32 combinations of initial refusal and sender-before/sample/
hold/sender-after faults. It retains first-error priority, peer construction
between the two sender checks, unix-seconds extraction only after the closing
check, refusal latching and no sender/time/peer work on refused reentry. Fake
field receivers replace live dependencies; one peer-observation expression is
reversed by the source canary to recover the complete original wrapper body.
This is a control-flow model, not validation of real time, sender liveness,
filesystem faults or a launch capability.

Source canaries compare the entire original collector and admission files with
Git, the full peer/wrapper bodies token for token, and the complete old allocator
file after reversing exactly one test hook. Mutation controls reject missing
push/dedup, truncation, sender/hold checks, latching or time extraction. Older
guards use the exact hook reversal; no live body or filename is exempted.

## Checks and first outcomes

The first Linux compiles refuse an ambiguous integer type in the seeded test.
The first host canary refuses the static-lifetime token used by case labels;
there was no static object. Those first sources and logs remain. The corrected
pure cohort adds an explicit usize and uses owned labels outside the counted
region; it does not relax the capability canary or alter the frozen operation.

Corrected checks pass: 34 source canaries; five new pure tests, nine earlier
pure controls and two counter controls per Linux compiler; affected all-target
check/strict all-feature clippy on host/GNU/MSRV; workspace format; doc-check;
unchanged 186-gate registry; and local non-promotable performance contract.
This is a focused step gate, not full milestone or ship verification.

The [packet](local-runs/collector-peer-baseline-20261021/summary.json) retains
both cohorts, initial and final source, methods and structural logs. The
[offline reviewer](../../../../tools/collector-peer-baseline-074/review.ps1)
checks stage conservation and nine earlier raw seals with 401 members, without
editing their frozen source-pinned reviewers. Qualification bytes remain intact.

The packet has 41 members plus its inventory; raw inventory SHA-256:
`cc19972e68a264b7b84ac48400f43e1b981d5a0e762ad7a9aee1a67548bf2452`.

## Next experiment and unchanged gates

The isolated next hypothesis is allocating room for the original supplementary
copy plus primary before push, then retaining the same sort/dedup and owned
Vec output. It needs its own preregistration, implementation and differential
comparison. Do not combine it with skipping membership work, changing limits
or caching a previous peer. No such candidate is implemented in this baseline.

All live-check counts/order, 300/60/10-second windows and first Causal(Time)
ages 10042390247/10010835739 ns remain. There is no live profile, clock/service
change, server/SSH/Actions, installation, supervisor or product workload.
Full material revalidation, cleanup/EOF/fence, authenticated delivery/all-writer
cessation, SIGKILL/lost-ACK/reconnect and server gates remain independent and
closed. Old trace counts cannot turn this baseline into current timing evidence.
