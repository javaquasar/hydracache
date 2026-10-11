# Original empty reservation fence local evidence

This W11 prerequisite removes the state-before-lock dependency for initial
diagnostic reservation. It was preregistered on 2026-10-11 and completed locally
on 2026-10-12. It is not a publisher, host enrollment, durable reservation,
worker start or benchmark. Baseline is
`b418c820e39f1c8e475cf6c73728409aaec314ab`; the
[contract](../../diagnostic-empty-reservation-fence-local-contract.toml) and
[design](../../diagnostic-empty-reservation-fence-local-design.md) precede code.

The guard exclusively borrows original fixed intent storage and reuses the
existing trusted campaign Fence engine. No Reserved file is required. Under
the original common flock, two independent descriptor-relative scans require
an empty fixed leaf, bracketed by retained ancestry/directory/lock and legacy
plus parent-context checks. Every entry refuses, without decoding or cleanup.
Initial Busy preserves healthy storage and the other owner. Any bound failure
latches the original storage through restoration/drop; later calls do no I/O.
Healthy drop releases only the retained owned lock. Empty observation does not
survive that drop and cannot become a write or execution capability.

## Receipts and proof boundaries

Capture 01 is the previous original-fence baseline. Capture 02 is the genuine
missing-public-API E0432 compile refusal under MSRV 1.88.0. This toolchain was
used for the absent-API probe because previous probes on 1.94.0 hit resolver
internal compiler errors; current 1.94.0 gates remain required. Capture 03 is
the first passing 11-case unit suite. A constructor-after-refusal check was
then added; final focused repeats are 04-06, not capture 03.

Cases cover both acquisition orders against legacy campaign APIs, repeated
empty scans, every fixed-leaf entry kind including non-UTF-8, all legacy/parent
contexts and dangling links, unsafe/missing/nonregular/hardlinked/nonempty locks,
deployed 0750 ownership refusal without repair, named/original descriptor drift,
missing/unsafe/replaced leaf, seed `0x749` between-scan context, sticky refusal,
non-Send/non-Sync and independent concurrent guards. External Linux API and four
new compile-fail examples establish the fixed-storage-only public boundary.

Capture 13 retains a root source-policy test failure: a single-line call marker
did not survive rustfmt's multiline call. The check now names the shared call
and original parent/owner arguments independently; no policy or acceptance
threshold changed. Corrected root regression is 23. Intermediate failures are
not relabeled as successful final evidence.

Final scoped repeats each pass 11 cases. Linux regression passes 552 with one
existing real-systemd-bus test ignored; Windows package tests pass 196.
Corrected root regression passes 157, including 115 performance-contract cases.
Linux compile-fail doctests pass 26. These are functional counts, not timings.

[manifest.json](manifest.json) retains exact commands, observed/expected exits,
raw hashes/byte counts, Git commit/tree/source bindings, repeated case names and
per-capture totals. Current/MSRV all-target/all-feature checks, warnings-denied
Clippy, format, rustdoc, Linux compile-fail doctests, native registry/governance/
contract checks, doc-check and native mdbook are separate receipts.
Windows zero-case Linux-gated targets are not Linux coverage, and the existing
real-systemd-bus ignored test is not counted as a pass.

Qualification dry-run and require-ship remain genuinely red. Frozen inputs and
qualification-manifest bytes are unchanged, including the inherited plan-digest
mismatch. No numerical claim or server operation follows this packet.

## Verification and next prerequisite

```powershell
C:/Python313/python.exe docs/testing/performance/0.74/local-runs/diagnostic-empty-reservation-fence-20261012/verify-packet.py
```

The verifier binds current source/tree, raw logs and W11 registration.
Historical intent/state evidence is checked against its own original source,
not today's extended API. All eight frozen qualification inputs are checked
without refreshing a digest or threshold.

Next join the same original empty fence with authority/signed expected intent.
A future publisher must transition under that retained lock into its explicitly
defined publication phase rather than treat empty observation as persisted
reservation or reacquire a replacement object. Define no-overwrite publication,
partial/uncertain/crash handling and the resulting retained state object before
mutation. Authenticated peer/clock and quiescent trusted ownership/coordinated
binary enrollment still precede a controlled original lifecycle pilot.
