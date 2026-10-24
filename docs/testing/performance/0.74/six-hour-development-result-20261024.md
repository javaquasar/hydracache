# Six-hour 0.74 development window: results and handoff

## Outcome and scope

The bounded window began at 2026-10-23 22:02 UTC and ends at 2026-10-24
04:02 UTC. Work stays on `feat/0.74-resp-native-throughput` in the separate
`hydracache-074` worktree. This is a development/diagnostic result, not release
admission. The release is **not ready for production or product qualification**.
No push is performed in this window.

The earlier explicitly scoped dedicated-Linux preparation/cost diagnostic is
retained separately. It expired before original delivery. No release was
installed; clocks, existing services and supervisor were not changed. No product
workload or expensive qualification was run. No additional server action follows
that diagnostic. Frozen 0.73 code and qualification artifacts remain untouched.

## Disk cleanup

Exactly 986 old, individually checked regular PDB files in this worktree's
`target/debug/deps` were deleted. Their total logical size was 27,100,012,544
bytes (25.24 GiB); the observed free-space increase was 10,643,259,392 bytes
(9.91 GiB), because many files were already compressed. No recursive directory
or worktree deletion was used. No executable, source, evidence or protected
experiment image was removed. The deleted symbol copies are not in the recycle
bin and cannot be recovered here; equivalent symbols require rebuilding.

Reversible NTFS compression of incremental data and PDB files also recovered
space earlier in the window. Those savings and the later deletion must not be
added as independent savings of the same files. Subsequent test builds consumed
some recovered space. Exact target manifests and before/after readings are in
the [local milestone packet](local-milestone-result-20261024.md).

## Implemented and checked

| Change | Evidence and limit |
| --- | --- |
| One-pass borrowed index of freshly read status fields | Five repeated parsing scans become one invocation-local scan; no live read/cache/order/deadline changes. 4096 seeded differential cases. Five paired pure-parser CPU comparisons per shape passed the preregistered gate. This is not total preparation or product throughput. |
| Separate requested-heap control | Four independently built GNU/MSRV images, 264 matched cases per compiler, 1056 parser executions. Exact transcript parity; ordinary three-group input retains one allocation/12 bytes and zero tracked bytes after drop. **No allocation reduction.** Not RSS or whole-runtime retained memory. |
| Exact historical source views and immutable Git lookup cache | Cache only successful complete pinned Git blobs, never live facts or current acceptance. Whole-source/mutation controls pass; all 321 selected performance contracts pass. Wider xtask package coverage has additional failures and is not green. |
| Owned subprocess scope for FD and bootstrap test oracles | Existing assertions and bodies preserved; timeout/stray descendants cannot pass. Seventeen bootstrap cases pass on GNU/MSRV. Root transport EOF successor passes six focused cases per compiler with one manual case still ignored. This is not production all-writer retirement proof. |
| Frozen material fixture input | Rejected re-signing with the current lock digest. Corrected the fixture to supply exact historical lock bytes without changing the signed receipt or production pin. Seven focused cases pass per compiler; foreign signed digest still refuses. |
| Independent ordinary correctness coverage | Windows RESP/client-surface: 251 pass/25 ignored across 23 targets. Embedded: 686 pass/25 ignored across 102 targets. Supervisor: 350 pass across 86 targets. Separate from performance acceptance. |
| Narrow context preservation reader | Exact reviewed registry is checked before its historical view; unknown drift still refuses. Sixteen focused source/control tests plus check/strict Clippy pass. No runtime code changes. |

The pure-parser upper 95% candidate/baseline CPU ratios were 0.2804685,
0.3180250 and 0.2357940 for the 512/2048/8192 shapes (fixed gate 0.85).
Do not project these ratios onto native API, full preparation or RESP throughput.
See [parser results](status-field-scan-result-20261024.md),
[heap parity](status-field-memory-result-20261024.md),
[ordinary repairs](ordinary-repair-result-20261024.md) and
[EOF scope](transport-eof-scope-result-20261024.md).

## First failures remain failures

The separate native diagnostic still exceeds its original ten-second window
before delivery. Saved inclusive spans identify repeated ReservedLive/kernel
work; nested costs must not be added. The changed complete ordinary Linux
cohorts remain red: GNU 1474 passed/8 failed/141 ignored; MSRV 1473/9/141.
Saved same-boot clock-order refusals are retained, not clamped or retried to pass.
Parallel log adjacency cannot assign every event to a particular test. No full
ordinary Linux rerun follows the later focused EOF scope repair.

The first standard `cargo xtask verify` stops at dependency policy with **15
advisories across ten packages**. Earlier format/clippy/structural lanes pass;
later gates in that invocation do not run. No exception or dependency update is
applied. The [security plan](dependency-security-plan-20261024.md) separates
compatible TLS/WebPKI/Tokio changes from YAML/ZIP migrations and the zbus 6
MSRV conflict (declared Rust 1.89 versus the current 1.88 contract).

The broader affected xtask package is separately bounded. Its initial registry
reader failure is repaired narrowly; the changed cohort retains sixteen failed
case lines and stops at its 780-second outer timeout with incomplete coverage.
Source/HEAD stay unchanged. See the [package frontier
report](package-frontier-result-20261024.md) for its exact partial outcome. A
targeted pass does not erase the wider red gate.

## Remaining dependency order

1. Repair the remaining exact-source contract frontiers with independent
   negative controls, without accepting unknown source drift. Resolve the
   dependency-security package and its explicit MSRV/migration decisions.
2. Obtain agreement for any change in ReservedLive observation frequency, then
   test unchanged entry/exit/effect predicates and original windows. The present
   leaf optimization alone does not prove complete preparation fits.
3. Select an explicitly approved compatible manager/host context and prove
   original eighteen-role delivery/readiness. The native host's systemd 255
   path expired before delivery; upstream version inspection is not live proof.
4. Integrate original worker release, autonomous watchdog and cessation of every
   writer/descriptor alias before durable completion or fence release.
5. Join completion/cells 2–4, uncertain-result non-reexecution, all new
   SIGKILL/lost-ACK boundaries, and same-attempt SSH/controller reconnect proof.
6. Approve actual policy/digest/epoch, authority pins and protected enrollment;
   build the exact signed final bundle/receipt and pass the complete local gate.
   Only then seek separate install/maintenance and nonproduct server-rehearsal
   authorization, followed by separately admitted pilot/qualification.

The [closure audit](release-closure-audit-20261024.md) is authoritative for those
boundaries. Qualification manifest SHA256 remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`,
state `prepared-not-authorized`, candidate identity unresolved and expensive
runs disabled. The explicit ship check still refuses as designed.

## Main changed paths

- Runtime parser: `tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs`
  and `diagnostic_projection_fields.rs`; differential controls in
  `diagnostic_projection_fields_tests.rs` and `tools/status-field-scan-074/`.
- Independent allocation method: `tools/status-field-memory-074/`; exact source
  gates in `tools/status-field-gate-074/` and `crates/xtask/tests/status_field_scan_074.rs`.
- Historical views/cache: `tools/historical-input-view-074/`,
  `tools/historical-source-cache-074/`, `tools/authority-runtime-074/` and their
  xtask source/mutation tests. Context-reader fix is only
  `crates/xtask/tests/authority_context_074.rs` plus its independent canary.
- FD/bootstrap/material oracles: `tools/owned-case-074/`,
  `tools/material-lock-fixture-074/`, supervisor root-transport/material tests,
  `tests/bootstrap_current_image.rs`, `tests/held_launcher.rs` and test support.
- Saved-result coordination/classification: `tools/native-server-control-074/`
  and `tools/ordinary-regression-review-074/`.
- Evidence/closure documentation: `docs/testing/performance/0.74/` and the
  optimization article `docs/articles/007-optimization-starts-with-measurement.md`.

Use `git diff --name-only d9a2b85c..HEAD -- crates tools docs` for the exact full
member list rather than treating these grouped paths as an exhaustive listing.

## Final structural verification boundary

Doc-check, local non-promotable performance-contract and the 188-gate registration
check pass. The final direct `cargo fmt --all -- --check` fails before formatting
with Windows OS error 206 (command/filename length); its first tool output is
retained as an explicitly non-raw receipt, not invented stdout. Separate direct
rustfmt of the two last changed Rust files passes. The earlier standard full
milestone's format lane passed at its own source identity. These observations
are not combined into a new complete workspace-format or full-verify pass.

The [final structural packet](local-runs/final-checks-20261024/index.json) retains
each bounded command's stdout/stderr/exit, executable hash, protected-input
before/after hashes and a read-only reconciliation of all twelve preceding
packets: 1685 members, 1386 physical objects and 707 exact common-source links.
Its eighteen members/thirteen objects have index SHA256
`9846bde26592d4a46d243f4468823cafaa70e82d2cb878d8c44f4cf5ee6fd976`.
At that capture the drive has 19,950,665,728 bytes free (18.58 GiB); this is
current headroom after intervening builds, not the isolated cleanup delta.

## Source and evidence commits in this window

Starting source: `d9a2b85c9a35ac8799f927f07e248e958918a7b5`.
Small logical commits, in order:

```text
db79fe40  separate native preparation-cost contract
2e7adf7a  partial native cost profile classification
bebcc0fc  native CPU/validation attribution evidence
3c52c0aa  parser baseline and paired method
81f4add2  fresh status-field runtime index
55269445  parser CPU results and first source refusals
533d4baf  ordinary regression first failures/FD counterexamples
11ad3be9  exact historical source-view composition
31ef668f  immutable lookup controls
5faae4c4  source/entry first outcomes
25faa7bb  immutable historical source cache
7f11119b  cache controls/product correctness evidence
e1f4be9c  owned FD-oracle subprocess scope
bcbf94d8  frozen hosted-lock fixture input
f5e59e3f  bootstrap oracle scope
5a5d983b  native build/ordinary execution coordinator
c025d341  saved clock-refusal classification
ac9b134f  ordinary repair evidence and closure audit
8c4008a5  allocation contract preregistration
079ee239  all immediate EOF root-transport oracles
7e6c8ee8  exact allocation/retained-requested-heap controls
4f3abd23  exact reviewed registry context reader
e2891d1e  allocation/EOF/full-milestone/package evidence packets
14c6efb3  final bounded structural checks/archive reconciliation
```

Final evidence/documentation commits follow this list in Git; no product crate,
root dependency lock or release-admission identity is changed by them.
