# Original intent and fenced Reserved state local evidence

This W11 packet joins original read-only authority/intent and state/fence
observations. It is not host enrollment, a publisher, diagnostic execution or a
product benchmark. Baseline is `0399d63bc1fb2020a26d522cc007d465ada59018`;
the [contract](../../diagnostic-fenced-intent-local-contract.toml) and
[design](../../diagnostic-fenced-intent-local-design.md) precede implementation.

Fixed and fixture wrappers exclusively borrow their matching original intent
reader and original fenced Reserved reader. Complete expected snapshots must
match before I/O. A wrong unbound assertion does not revoke healthy inputs.
Revalidation observes fence/intent/fence/intent/fence, retaining the existing
brackets within each original chain. Bound failure latches both inputs.
Fence failure publishes Policy refusal only to independently exact-safe
original authority storage; unsafe or advanced authority is not appended.
The first error and refusal uncertainty remain typed. Intent errors reuse their
existing refusal bridge without duplicate publication. Already refused calls
perform no I/O, and successful wrapper drop leaves the original caller's flock
held.

## Receipts and source boundaries

Capture 01 passes the 13 existing authority/intent tests. Captures 02 and 03 are
Rust 1.94.0 resolver internal compiler errors during absent-API probes, not
semantic expected-red proofs. Capture 04, using MSRV 1.88.0, is the genuine
missing-API E0433 result. No dependency, toolchain pin or threshold was changed
to mask the compiler failure; final current-toolchain gates remain required.

Capture 05 retains a test import ambiguity, corrected by explicit Step/bracket
imports. Captures 06 and 07 retain a durability-test error: reopening authority
while its original registry still owned the lock. Reopen was moved after all
original authority guards dropped, preserving production exclusion. Capture 08
passes the expanded 14-case suite. Captures 09-16 are intermediate because a
second before-I/O state-mismatch case was added afterward; they are not
substituted for final scoped repeats and Linux regression 31-37.

Final tests cover healthy repeated observation/drop, both full-state mismatch
branches, bound signature/state/peer/expiry changes, original state inode drift,
seed `0x748` late context failure, constructor failure, unsafe/advanced authority,
partial refusal publication, no-I/O after either original chain refuses,
fixed/fixture distinction, non-Send/non-Sync types and independent concurrency.
Positive fixed dispatch uses a private nonroot model, not a production path or
operator issuer. Four new compile-fail examples cover borrowing, cloning,
promotion and absence of a start conversion.

Each final focused repeat (31-33) passes 14 cases. Final Linux regression (34)
passes 540 cases with one existing real-systemd-bus test ignored. Windows package
regression (17) passes 196 cases; native root contract/registry/governance tests
(18) pass 156, including 114 performance-contract cases. Linux doctests (13)
pass 22 compile-fail cases. These are functional counts, not performance metrics.

[manifest.json](manifest.json) records exact commands, observed/expected exits,
raw byte counts and SHA-256, test totals and Git source/tree bindings. Checks
include warnings-denied Clippy, all-target/all-feature current/MSRV, format,
rustdoc, Linux doctests, native root registry/governance/performance contract,
doc-check and native mdbook. Windows Linux-gated targets that run zero cases are
not counted as Linux coverage. The real-systemd-bus test remains explicitly
ignored, not claimed as a pass.

Qualification dry-run and require-ship remain genuinely red. All eight frozen
qualification inputs and qualification-manifest bytes are unchanged; the
inherited main-plan digest mismatch is not repaired here. This packet grants
no server operation, worker execution, qualification or numerical claim.

## Verify and next prerequisite

```powershell
C:/Python313/python.exe docs/testing/performance/0.74/local-runs/diagnostic-fenced-intent-20261011/verify-packet.py
```

The verifier binds current implementation to its recorded Git source/tree,
original raw captures, W11 registration and documentation links. Historical
fixed-admission evidence remains bound to its own source, not today's extension.

Sequential observations do not prove continuous confinement, authenticated
initial peer/clock or physical host enrollment. Coordinated common-binary
deployment and quiescent trusted ownership still precede trusted state and
intent publication. Reservation/consumption publication and original
output/start/watchdog/owned cleanup remain before a controlled pilot.
