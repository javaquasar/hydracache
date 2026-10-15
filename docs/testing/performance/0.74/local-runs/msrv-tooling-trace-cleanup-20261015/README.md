# Tooling MSRV and loaded trace cleanup

This local step addresses the eight source locations enumerated by the
[conditional diagnostic packet](../msrv-conditional-diagnostics-20261015/README.md).
The baseline is `55e4e5f7eaf230d48527330c6a884f5bef162871`.

Seven changes inline existing format arguments in loadgen and xtask. Archive
names, CSV diagnostics, in-flight mismatch diagnostics, canary/fast-suite
messages and bootstrap fixture timestamp padding must remain unchanged. No
workload, seed, estimator, duration, measurement threshold or scenario changes
are in scope. Characterization fixtures are tested before the interpolation
changes. They create only local synthetic data or exercise scripted executors,
not Redis or product qualification workloads.

A separate private supervisor change collapses `else { if marker ... }` into
`else if marker ...` in `loaded`. The READY-write branch must retain priority;
marker validation, getpid/kill branches, startup allowlist, ptrace/wait order,
256-event budget, five-second process guard and cleanup remain unchanged. Real
public-dummy process diagnostics are run separately from heavy compilation.
This is not authenticated original-start, protected credentials, all-writer
cessation, completion/later cells or server admission.

The initial strict Windows MSRV check fails on three loadgen expressions; Linux
fails first on the nested supervisor branch. These failures and the complete
baseline tests are retained rather than replaced by later results. Existing
historical packets keep their original source identities and failed outputs.

## Source identities

Format/test commit: `06579cb21ce9bb6aa2266fd49505bb600870c416`.
It changes nine loadgen/xtask source and test files. Trace-only commit:
`8a22f2199e2759681bc47f88746cf972a33345ee`; its parent is the format commit,
and its only changed file is the private root-handoff source.

Baseline logs use the original source. Test-first logs and patch add the five
fixtures before changing formatting. Format compiler/runtime logs precede the
format commit on its corresponding changed files. Final compiler logs and the
first process negative use the trace edit before its commit. The original-trace
control restores only that edit, matching the format commit's supervisor source;
the subsequent trace repetition restores exactly the registered rewrite.
Documentation/evidence is dirty during captures. These are source associations,
not signed or clean-worktree execution attestation.

## Format characterization and compiler lanes

Five new tests pass before changing the seven interpolations. They pin archive
names for decimal counts 0/12 and raw hard-link bytes, the exact rejected CSV
header diagnostic, both missing-plan/canary messages, the reviewed 1680-second
ceiling diagnostic, and two-digit fixture dates for 1/9/10. The budget test changes
only an in-memory fixture; the real budget and registry are not edited. Archive
name inputs do not establish node-count admission. Existing archive replacement
and duplicate-refusal coverage remains in place.

After the interpolation changes, Windows Rust1.94 passes 60 focused tests:
seven metrics/archive, 22 CSV/external scripted-contract, four pure client-surface,
13 canary, eight fast-suite and six bootstrap tests. The external executor cases
use scripts/fakes, not a Redis performance run. The client-surface measurement
smoke validator is explicitly skipped, not counted as passing. The in-flight
mismatch branch is not forced through a measurement run; its exact source rewrite
is checked separately. No Linux/MSRV runtime repetition of these 60 is claimed.

Loadgen/xtask all-target checks and strict Clippy pass on Windows/Linux with
Rust1.88/1.94 before the trace change. After that separate change, the wider
supervisor/xtask/loadgen all-target, all-feature checks and strict Clippy pass
in all four combinations. Whole-workspace Rust1.94 formatting also passes through
WSL. This closes the observed eight-site strict lint inventory, not an exhaustive
workspace, allocator, platform, feature, runtime-support or release qualification
matrix. No lint suppression, serial process-test override or guard widening is
used.

## Process negative retained

The original Linux Rust1.94 supervisor group passes 67 tests before the trace
edit. The first after-edit group passes 66 and fails one: the handoff integration
test reaches its unchanged five-second `normal root preparation stalled` guard
before its first successful-good-mode receipt. Its full output is retained as
[after-trace-process-timeout.log](after-trace-process-timeout.log). No separate
heavy compilation is deliberately running alongside this group. This is not
described as another proven compiler-overlap failure. The generic guard message
does not identify the stalled syscall or establish its cause. Compiler success
does not waive this process failure; control and repeat results must be reported
separately.

Restoring the original trace branch passes the same 67-test group. Restoring
the single registered trace edit then also passes 67, including all public-dummy
handoff, stopped creator-loss, healthy and no-EXITKILL controls. Default parallel
test scheduling and the five-second per-process guard remain unchanged in both
runs. This establishes an intermittent observed failure, not its cause or a
proof of load robustness. The successful control cannot establish that the
source edit caused or did not cause the first timeout.

A second trace repetition after commit `8a22f2199e2759681bc47f88746cf972a33345ee`
also passes all 67. Thus the packet has two original-trace successes, one trace
candidate 66/1 failure, and two later trace successes. None replaces the others.

## Structural and evidence boundaries

[source-rewrites.json](source-rewrites.json) records the seven exact format
rewrites and the one trace branch collapse. The format comparison excludes added
test bodies. The supervisor comparison covers its whole file and checks that no
other supervisor file changed from the baseline. It preserves READY-write
priority, all marker/getpid/kill/startup conditions, ptrace/wait ordering and
the event budget. This is unsigned structural consistency, not runtime execution
attestation or a proof that every branch ran.

The 165 static performance-contract tests and 175-gate registry check pass.
The local contract is non-promotable; require-ship still refuses with exit1.
Cargo.lock, the qualification manifest and the three preceding retained negative
captures keep their original SHA-256 values. New tooling source identities do
not rebind earlier qualification receipts. No performance result, allocation
improvement, product candidate, workflow, server launch, merge/tag or push is
part of this packet.

[manifest.json](manifest.json) pins commands, exit statuses, source/tree/blobs,
all raw captures, this README and the rewrite specification. Baseline strict
failures and the process timeout retain expected gate exit0 and actual exit1;
only require-ship is an expected refusal. `verification.txt` is the separate
unsigned consistency transcript, outside the manifest's capture set. The
reproducible consistency command is included in the manifest; neither hashes
nor its transcript are signed execution evidence.

The first consistency attempt refused an omitted `trace_commit` field in the
draft manifest. The original JSON and full refusal are retained separately as
`manifest-first-failed.json` and `verification-first-failed.txt`, included as
extra artifacts rather than successful runtime/check captures. The field is
corrected from the pinned source identity; source code, commands and runtime
results are not changed. The successful final consistency transcript does not
erase that metadata-construction failure.

Next: identify the intermittent handoff guard stall without changing bounds or
test scheduling. Then continue trusted runtime/issuer/job custody, authenticated
original-start and P2/P3 cessation of every writer before completion/later cells
or any separate server admission. No numerical performance claim follows from
this compiler/process diagnostic step.
