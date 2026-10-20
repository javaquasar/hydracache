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

No server run, installation, supervisor, clock/service operation, product
workload, native matrix retry or full qualification occurs. Separate processes
or boots do not establish clock continuity. Existing native time-order refusals
and lifecycle/trust/writer retirement/reconnect gates remain independently closed.
