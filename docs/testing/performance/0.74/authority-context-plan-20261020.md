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
