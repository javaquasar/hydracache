# Authority binding context ownership control

This local experiment removes only temporary context ownership in authority
`Record::bind`. Issuer, policy and epoch predicates retain their prefix order;
context fields retain machine, boot, user namespace and mount namespace equality
order. The existing `context()` method and parser/history callers stay unchanged,
as does the previously accepted issuer-key encoder. No live result is cached.

The [preregistered plan](../../authority-context-plan-20261020.md) fixes three
binding arms: frozen owned-context reference, proposed field comparison, and
actual runtime binding. Healthy, issuer-mismatch, epoch-mismatch and
context-mismatch cases each have 32 warmups and three 1024-operation samples,
rotating arm order on GNU 1.94 and MSRV 1.88. Prefix-mismatch cases also carry
a wrong context, proving that their rejection does not pay the context clone
cost. Captures count successful Rust requests, not libc calls or retained memory.

Each Before/After stage independently runs the unchanged eight-case complete
fixture inspect/Origin/leaf control. Allocation, kernel-walk and I/O captures
do not overlap. Complete inspect must conserve the isolated healthy binding
reduction at both one and seven journal generations. Reallocations may remain
unchanged; no earlier test threshold is changed to force an improvement.
All non-authority cases and kernel/read/write request counts must remain equal.
Fixed production ancestry and full ActiveAuthority latency are outside this
fixture. Counter-reader overhead stays in raw logs and is subtracted only from
read request counts, not converted into workload or disk bytes.

`check.ps1` records native exits and hashed logs. The initial new source canary
rejects method/free-function indentation, before any runtime candidate or
measurement. `xtask-canary-refused.rs`, the failed log/receipt and
`canary-refusal.json` retain that result. Its correction accounts for the exact
four-space scope difference; no old assertion or helper/runtime code is changed.
`repair-check.ps1` records separate successful Before xtask checks without
overwriting the refusal. It is not a repeated measurement cohort.

`profile.ps1` binds clean source, recipe, normalized committed input bytes and
compiler image before/after hash. Its two probes execute serially; images are
rebuildable from source/toolchain identities, not stored in Git. `review.ps1`
recomputes results and conservation, checks all accepted receipts plus the
initial refusal, rehashes nine earlier packets and exercises summary mutations.
Once sealed, never edit or reseal this packet.

`final-check.ps1` separately requires identical committed blobs and normalized
SHA256 for the reference, binding workload, unchanged complete inspect control
and qualification manifest across Before/After on each compiler. It records
final documentation, registry and local performance-contract checks after the
results are documented. It does not replace any check or measurement recipe.

No server run, installation, supervisor, clock/service operation, product
workload, native matrix retry or full qualification occurs. Separate processes
or boots do not establish clock continuity. Existing native time-order refusals
and lifecycle/trust/writer retirement/reconnect gates remain independently closed.

## Recorded result

Before source is `acdcdc30bbd37641ac06440d0aa2b9ec75908711`; runtime After is
`b8e02d6aee043e6d39d1719d8dcc9f44706aff03`. All three samples on both compiler
lanes agree. Counts below are successful Rust requests per operation, with
requested bytes including realloc sizes, not retained memory.

| Operation | Allocations Before to After | Reallocations Before to After | Requested bytes Before to After |
| --- | ---: | ---: | ---: |
| Healthy or context-mismatch complete binding | 3 to 1 | 0 to 0 | 132 to 64 |
| Issuer/epoch mismatch, also wrong host | 1 to 1 | 0 to 0 | 64 to 64 |
| Complete fixture inspect, one generation | 37 to 35 | 8 to 8 | 2710 to 2642 |
| Complete fixture inspect, seven generations | 153 to 151 | 56 to 56 | 12988 to 12920 |

Every complete inspect sample conserves the isolated healthy binding reduction:
two allocations, zero reallocations and 68 requested bytes. Reads remain four
and 28 at the respective depths. All six non-authority leaf cases and every
fresh kernel count remain equal; measured batches perform no write requests.
This accepts only the bounded request substitution. Full ActiveAuthority,
CPU, latency, retained memory and cache API improvement remain unmeasured.

Before boot is `d565c7a2-36c0-4d67-a4c9-fa8dd2440d38`; After boot is
`b4b655c5-f2f0-446e-9fc2-378681ae0f80`. They are independent cohorts, not a
continuous process/campaign or clock-stability proof. No live matrix is retried.

Both stages pass authority 189/189 per compiler with 11 ignored manual tests,
focused kernel/trust/allocator controls, check and strict clippy. All 367
historical/current source contracts and format/docs/registry/local performance
checks pass. Frozen reference/workload and qualification inputs remain equal.
`summary.json` recomputes stage-to-arm identity and conservation from raw logs;
integrity/summary mutation passes do not grant native or server admission.
