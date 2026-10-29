# Applied-root D-Bus package gate (0.74)

This stage applies the exact 291-operation root candidate retained by
`../dbus-root-integration-074/`. It checks the actual root package with GNU Rust
1.94 and MSRV 1.88. It grants no release, production, systemd, or server authority.

The source packet is pinned by `apply.INDEX`; every replacement requires its
original preimage, and every addition must be absent. Formatting is restricted to
the reviewed migration sources, followed by a complete workspace format check.
The earlier Windows OS206 and GNU toolchain-override refusals are retained.

`freeze.py` captures the applied/formatted source, all 90 package targets, and all
644 reviewed registry archives. The native seed uses only checksum-bound local
archives and a private Cargo directory source. It changes no global Cargo/Rustup
configuration. `review_runtime.py` retains the original freeze and seals the
explicit pre-execution deadline hardening separately.

The fixed phases in `run.py` are `seed`, `format`, `checks`, `focus-194`,
`focus-188`, `spool-194`, `spool-188`, `tests-194`, `tests-188`.
Each has its own exclusive output directory, raw stdout/stderr, original Windows
Job, LinuxGroup retirement, 4 GiB floors, and fixed 1200/1100-second outer/session
bounds. No phase is automatically retried. The first completed result remains
authoritative for its exact source/helper context.

Package checks retain `--all-targets --all-features --offline --locked`, with
strict Clippy `-D warnings`. Tests build every original package target and execute
only actual test-profile artifacts. The 1,593-case library catalog runs in four
independent single-case processes; `--color=never` preserves the original
owned-case outer 45-second/capture/stray frame. Every case occurs once, with its
own original LinuxGroup. Local per-case filtering is accounted separately from
global omission, which must be zero. All nonlibrary harnesses remain serial and
unfiltered. The test working directory matches Cargo's package directory.
Ignored tests stay distinct; an additional explicitly selected private-bus matrix
exercises ten synthetic transport cases without a real manager/service. Normal
executables are never treated as test harnesses. Both compiler catalogs must
match before claiming a complete paired package test result.

The standalone `source_guard.py` is an inert CI source-custody candidate, not a
runtime validation change. Each Linux file read uses a fresh `openat2` traversal
below the held root with `RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS |
RESOLVE_NO_MAGICLINKS`, checks that the opened FD is regular, reads every byte,
and closes the FD. Unsupported kernels fail closed. There is no digest cache or
fallback. `source_control.py` checks this separately without modifying a running
package attempt. Integrating it requires a separately retained helper context.

The applied-root freeze binds 2,246 source/data inputs, including 13 external
compiler includes and the additional SHA-256 regression source. One confirmed
bootstrap-cost leaf uses the existing pinned ring SHA-256 for both complete-file
consumers, retaining every fresh read, predicate, metadata check and original
window. It changes no dependency. One synthetic spool fixture uses the existing
exact historical worker lock instead of the moving current root; actual trust
pins and signature verification remain unchanged.

Historical contexts begin at `target/dbus-root-package-20261028/`; the complete
accepted context is `target/dbus-root-package-20261029-formatted8/`. Full GNU
formatting, paired all-target/all-feature check and strict Clippy pass there.
Both 17-case bootstrap and seven-case spool focuses pass; both complete catalogs
have 2,105 passed / 136 ignored / 90 targets, plus the additional private matrix.
No earlier partial result is reused. Only the immutable checksum-verified
registry seed is reused in the final context. Earlier timeout, caller-mode,
capacity, hash-baseline, stale-fixture, formatting and preparation-only refusals
remain explicit. Missing early outer streams are labeled, not reconstructed.

`retain.py` preserves every context and exact tested source under
`docs/testing/performance/0.74/local-runs/dbus-root-package-20261028/`. Its index
SHA-256 is `33cfe7ec5061690fa0f03ab5d81df4a598d7a8a810ffc4ae7457fb4bc97d24d9`.
Built binaries and registry archives are excluded; full archive checksum and
decoded-source receipts remain. The independent review producer/raw results live
in the adjacent `dbus-root-package-review-20261029/` directory. Nineteen final inert
controls also cover byte/text preimages, metadata hashes and formatter edition.

These are one-shot, preimage-bound historical preparation scripts. After retaining
their exact executed bytes, the working metadata helper is corrected and its
regression-tested version retained separately. Current README/helper bytes are
therefore not the old runtime freeze. Completed phase directories and old freezes
must not be removed or replayed to get a pass. A new change needs a new reviewed
source/helper context; a historical freeze must fail closed on helper drift.
Never commit during a gate: HEAD and tracked status are part of source custody.
Product admission windows remain 300/60/10 seconds and test bounds 5/10/45, with
all live predicates and their observation frequency/order unchanged. Broader
workspace/security, authenticated original execution and server admission remain
open. No product performance claim follows from these compatibility results.
