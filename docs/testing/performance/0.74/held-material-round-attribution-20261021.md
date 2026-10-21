# Repeated validation round cost in the first materialization refusals

Offline attribution of the two [retained first refusals](held-material-live-20261021.md)
identifies repeated eight-step rounds, especially `ReservedLive`, as the largest
measured part of the new combined phase. It does not turn either refusal into a
pass or establish a speedup. Runtime, observer, all live predicates and original
300/60/10-second windows remain unchanged; no new profile, WSL session, server,
clock or service operation is performed.

The analyzer is frozen at `5536087e7ad1acfa1af66f796631bdfb4338d2e0`. It reads
only the source-`1d6d179b` packet pinned by inventory SHA256
`0c385fbfe209fad1903aa2c0e620817c458f413eec8bd36fa32be5c01fbb4109`, verifies all
65 members and reconstructs the sealed summary with its original reader.
Derived facts live in a [separate packet](local-runs/held-material-round-attribution-20261021/README.md);
no old packet, recipe, refusal or result is edited.

The derived packet contains 18 members plus its inventory, whose raw SHA256 is
`84c2d0a10e1d2d66b4dc76e1c041b9920d90d85621461bc3e83102569a6e1a2f`.

## What the logs establish

Across all eleven observed phases, GNU starts 130 eight-step rounds and completes
129; MSRV starts 133 and completes 132. The final projection/materialization
phase starts 56/59 and completes 55/58 respectively. Both unfinished passes
stop at Causal, with no continuation to Prepared or the remaining closing steps.
A round here means one pass through the eight instrumented steps, not one
high-level API revalidation or necessarily a complete two-sided validation frame.

The first expired admission ages remain 10042390247 ns and 10010835739 ns. The
same three material markers remain: projection entered, projection returned and
materialization returned with twenty descriptors. Revalidation/drop success
markers and the successful cleanup footer remain absent. Construction succeeded,
but full material admission and final EOF/fence assertions remain unproven.

| Final combined phase wall component | GNU ns | MSRV ns |
| --- | ---: | ---: |
| Kernel spans inside eight-step rounds | 2061329069 | 2153387665 |
| Other wall time inside those steps | 1210233202 | 1272130444 |
| Kernel spans outside those steps | 380446764 | 403459167 |
| Unassigned wall time outside those steps | 766917781 | 824444588 |
| Exact phase total | 4418926816 | 4653421864 |

The eight-step portions total 3271562271 ns and 3425518109 ns, or 74.04% and
73.61% of these observed phase prefixes. Within them, ReservedLive alone takes
2808784775 ns and 2941286486 ns: 63.56% and 63.21% of the entire combined phase.
Its 56/59 calls all finish; the later Causal refusal is not reassigned to this
dominant step. No individual round latency distribution or stage timestamp is
available, so these are aggregate costs, not p50/p95/p99 or projection timings.

## Fresh kernel checks dominate the measured kernel tree

Within instrumented round steps, Boot, Namespace and Origin together contribute
1471456723 ns and 1537508629 ns outside their instrumented children, about 71.4%
of the corresponding kernel subtotal. Their observations remain fresh.

| Kernel kind inside round steps | GNU calls | GNU wall ns outside children | MSRV calls | MSRV wall ns outside children |
| --- | ---: | ---: | ---: | ---: |
| Boot | 186040 | 598744433 | 196024 | 637168821 |
| Namespace | 372080 | 511601106 | 392048 | 528083764 |
| Origin | 93020 | 361111184 | 98012 | 372256044 |

The complete phase also contains 34088/36512 Boot observations and 68176/73024
Namespace observations outside the eight step scopes. These are retained as
outside-step checks, not mislabeled as material effects. The remaining kernel
kinds, root/edge counts and per-step durations reconcile exactly in the facts.
This ranking concerns the already instrumented synthetic path, not the native
cache API, an exclusive CPU profile or a comparison between compilers.

Instrumented message authentication occupies 51794036 ns and 56034103 ns in the
whole final phase, about 1.17% and 1.20%. Of 1106/1168 authentications, 952/1003
occur inside the eight-step scopes and 154/165 outside. Signature spans are
nested inside authentication; neither is added to kernel or step wall time.
This does not measure all cryptographic work in trust/receipt/role checks.

## Why wrappers revisit the same deep path

Source inspection explains a repeated successful-path call structure, without
assigning measured time to unobserved stage boundaries:

| Caller | Fresh calls it preserves |
| --- | --- |
| validate_material | Two full_frame calls, each using projected-owner revalidation |
| Projected owner round | Two complete consuming-holder validate calls |
| Consuming-holder validate | Four placement frames, two trust inspections and two role inspections |

Each layer deliberately brackets different retained state and effects. The
multiplicity is visible in `diagnostic_held_unit_material.rs`,
`diagnostic_held_unit_projection.rs` and `diagnostic_consuming_held_join.rs`;
the lower eight-step observations are in `diagnostic_validation_frame.rs`.
These are source-derived call relationships, not extra observed stage samples.
Collapsing the calls would change observation frequency and is not an accepted
optimization from this analysis. Caching a prior live success, rebasing an
origin or extending the ten-second window is not permitted.

## Arithmetic and interpretation boundaries

For each of 22 observed phases, the analyzer verifies every step prefix, kernel
node, root and edge with the unchanged arithmetic reader. For every kernel kind
it checks that phase counts, inclusive time and time outside children are at
least their in-step sums, then retains the exact difference. Negative residues
refuse analysis; none are clamped or hidden. The four components above add to
the phase wall time exactly. This partition is within the instrumented tree:
outside-children wall includes anything not separately instrumented there,
including observer cost and waiting. It is not exclusive kernel CPU time.

The 766917781/824444588 ns unassigned outside-step residue cannot be called
fsync, memfd, allocation, census or observer cost. It also contains unseparated
comparisons, hashing, preparation and failure unwinding. Parent CPU values are
preserved even when greater than phase wall; they are not clamped or used as
per-step CPU estimates. Summed observed phase wall, 9677187411/9682299324 ns,
excludes inter-phase work and report emission and is not the original admission
age or a remaining-budget calculation.

Fifteen new offline controls pass, plus nine existing material-reader controls
and eighteen existing arithmetic controls. The initial synthetic residual test
was rejected by the earlier live-wall invariant instead of its expected residual
guard; its log and source remain retained. Only the synthetic test oracle was
adjusted to reach the later guard. The analyzer, input arithmetic and live runtime
were not relaxed to repair data. All 22 real phase partitions already conserve.

Fourteen existing source canaries pass and reconstruct the unchanged runtime
and live checks. Windows xtask all-target check and strict all-feature clippy,
doc-check, the 186-gate registry and local non-promotable performance-contract
check pass. No Rust/product implementation changes or Linux test/profile runs
are needed for this offline step. The original input seal remains unchanged.

Next investigate implementation cost inside the repeated ReservedLive fresh
worker/authority and Boot/Namespace/Origin inspections, preserving the same
observations and original deadlines. Treat this as a target for a separately
reviewed candidate, not a predicted saving. Material-effect and observer
subtraction require measurements absent from these logs. Full material admission,
delivery, all-writer cessation, SIGKILL/lost-ACK/reconnect and server gates remain
closed; the first refused cohort is never rerun to obtain a pass.
