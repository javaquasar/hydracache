# Authority context ownership control

Baseline is `a0209ce87c72d6553ad152d49f88abb61a10696e`, including the accepted
issuer-key encoder substitution. Complete fixture inspect currently requests
37/153 allocations at one/seven generations. Only temporary context ownership
inside `Record::bind` is a candidate: replace construction of an owned
AssertedWorkerHost for equality with borrowed field comparisons in the same
machine, boot, user namespace and mount namespace order. Keep issuer, policy
and epoch prefix checks and their short circuits unchanged. Leave `context()`
itself and its parser/history callers unchanged.

First add frozen-reference, proposed and actual complete binding controls.
Seeded differential cases cover healthy contexts, every independent pin and
namespace field, simultaneous mismatches, canonical bytes and refusal class.
Measure healthy, issuer-mismatch, epoch-mismatch and context-mismatch bindings
separately. Each arm has 32 warmups and three 1024-operation samples on GNU 1.94
and MSRV 1.88. Prefix mismatches must not pay the context ownership cost in any
arm; healthy/context-mismatch may remove only the two temporary String clones.
Reallocations are expected unchanged, not required to improve.

Retain Before measurements and tests before the one-expression substitution.
Repeat the same controls After. The complete fixture inspect reduction at both
one and seven generations must equal the isolated healthy binding reduction.
All six non-authority leaf cases and kernel/read/write request counts must stay
equal. No temporary names, inventory, source traversal, observation reuse or
other ownership change belongs in this candidate. Keep the prior encoder
reference, checks and evidence unchanged.

For each stage run focused tests, affected-package all-target check and strict
all-feature clippy. Before final measurement run historical source/contracts,
format, docs, registry and local non-promotable performance checks. Freeze
source, recipes, images and workload identities; keep native exits and raw logs.
Never overwrite or retry a failed compiler cohort into healthy observations.
Rehash nine preceding sealed packet anchors. If conservation/correctness fails,
retain the failure and revert only this candidate.

These are successful Rust allocation/reallocation requests and requested bytes,
not libc calls, retained memory, CPU or elapsed-time measurements. The authority
fixture omits fixed production ancestry and the full ActiveAuthority wrapper.
Do not infer live clock continuity across separate processes/boots. Existing
native clock-order refusals remain closed; no new native matrix is authorized
by a successful allocation control.

All work is local. Preserve all live checks, their frequency and 300/60/10
deadlines. No server, clock/service control, installation, supervisor operation,
product workload, full qualification or push through the red live gate.

## Local outcome

The one-expression runtime substitution is `b8e02d6a`. Frozen Before source is
`acdcdc30bbd37641ac06440d0aa2b9ec75908711`; After source is
`b8e02d6aee043e6d39d1719d8dcc9f44706aff03`. The
[paired packet](local-runs/authority-context-20261020/README.md) retains three
samples of each binding case and unchanged eight-case inspect control on both
compilers, without retrying a measured cohort.

Healthy/context-mismatch binding requests change from 3 allocations, zero
reallocations and 132 bytes to 1/0/64. Issuer/epoch prefix refusals remain 1/0/64,
even with a wrong host. Complete fixture inspect changes from 37/8/2710 to
35/8/2642 at one generation and 153/56/12988 to 151/56/12920 at seven. The
2/0/68 reduction conserves in every sample; reads stay 4/28 and all six other
leaf cases and fresh kernel counts remain equal. The candidate is locally
accepted for requests only, not CPU, retained memory or full-frame latency.

Both stages pass 189 authority tests on each compiler (11 manual/privileged
tests ignored), focused controls, all-target checks and strict all-feature
clippy. All 367 historical/current contracts and repository checks pass. The
initial new source-canary indentation refusal is retained with its separate
repair; it precedes any measurement and changes no prior assertion.

Before/After report different WSL boot IDs; neither live continuity nor the
old clock-order/native gate is repaired. The next representation hypothesis
must isolate remaining parse/context, name or inventory ownership, one at a
time, before another substitution. No live check, its frequency, 300/60/10
deadline, qualification identity or independent execution/trust/lifecycle gate
changes. No push, server or clock/service operation occurs.

Evidence commit is `b3446db5e24e0439be6be868774b406b72839b9d`, with 106
members plus inventory SHA256
`25796efbc05cbcca3a7c646d70624774979c84fb16c4beb84746a4c0cc70a01f`.
Nine earlier packets rehash and ten summary scope mutations refuse. The sealed
packet is immutable; further hypotheses require a separate preregistered packet.
