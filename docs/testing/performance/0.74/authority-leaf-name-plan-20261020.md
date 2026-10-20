# Authority leaf name ownership control

Baseline is `8a605558e8e773df2c0e6f09bfa23364ada158f1`, including the accepted
issuer encoder and binding context changes. Source audit defers parse/context
ownership for the present repeated-inspect owner: `history` parses on opening,
while `WorkerTrustInput::active` calls retained `FixedAuthorityRegistry::inspect`
without reparsing. No parse saving may be credited to that live chain.

The single candidate is temporary name ownership in `Leaf::revalidate` inside
`diagnostic_authority_fixture.rs`. It clones a retained record/commit name only
to reopen and compare fresh file metadata and bytes. Extract the unchanged
checked read into a private borrowed-name helper. Initial opening still owns
the name; revalidation borrows it only for the synchronous read. Keep the newly
opened File alive through comparison, and preserve every check, read capacity,
error order, inventory, original-owner check and original 300/60/10 deadline.

Before runtime mutation add frozen owned-open/reference revalidation, proposed
borrowed-read and actual arms. Seeded controls cover record/commit names and
bounded payloads, original metadata drift, name loss/replacement, unsafe modes,
links, budget and ownership refusals. Measure record, commit, early metadata
refusal and missing-name cases with 32 warmups and three rotated 1024-operation
samples on GNU 1.94 and MSRV 1.88. No runtime candidate is adopted on leaf
counts alone. Retain Before source/images/logs first, then substitute only this
name ownership and repeat the checks and observations.

Run the unchanged eight-case complete inspect/Origin/leaf control independently.
At one/seven journal generations the full inspect request reduction must equal
the measured record-plus-commit reduction multiplied by one/seven, respectively.
Reallocations, all six non-authority cases, read/write and kernel-observation
counts must remain equal. Early refusals must retain their class and short
circuit. Counts describe successful Rust requests and requested bytes, not CPU,
latency, retained memory, libc calls or full ActiveAuthority cost. Names and
fixture payloads/reference/control sources must be identical across stages.

Each stage requires focused tests, affected-package all-target check and strict
all-feature clippy. Before final measurement run all historical/current source
contracts, format/docs/registry/local performance checks. Freeze recipes and
committed inputs/images; retain native exits and never overwrite or retry a
failed measured cohort into healthy results. Rehash the ten preceding sealed
packets; never alter them. If conservation/correctness fails, retain the result
and revert only this candidate. The initial source audit is not a numeric parse
measurement or a general assertion that cold preparation cannot be optimized.

All work is local and nonroot synthetic fixtures. No new server/live-matrix run,
clock/service operation, supervisor control, installation, product workload,
qualification or push through the old red live gate. Separate processes/boots
do not prove continuity. Original delivery, all-writer cessation, later cells,
SIGKILL/lost-ACK, reconnect and real enrollment remain independent closed gates.

## Test hook placement amendment before After capture

The first final historical gate refuses because the new test module hook
precedes runtime, while one unchanged old assertion reads only the source
prefix before its first `cfg(test)`. The journal lock and other predicates are
present. Move only that identical hook after existing test declarations and
retain the complete failed gate/source identity. Runtime, old assertions,
reference, workload, profile recipe and all thresholds remain unchanged.
Repeat all After compiler/xtask/historical/repository checks into distinct
repair logs and freeze the repaired test image before the first After capture.
No measured After invocation has occurred; Before observations are not retried.
Keep original recipes unchanged. The separately frozen After-only profile
selects repaired build/check receipts with identical probe selectors, inputs,
iterations and capture behavior; the repair reviewer validates both recipe
identities and retains the failed historical gate.
