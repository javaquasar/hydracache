# Repaired GNU package review and complete red gate

The distinct GNU command completes all 155 default package targets on source
`fe4b3a0f9060615185895337181780a6f3f55439`: 1,340 passes, two failures, zero
ignored cases and no missing suffix. All ten failures from the previous complete
GNU result now pass. Two independent current-tree audit cases reject four newly
changed source bodies. The package gate remains red; no unchanged full retry,
merge, tag or release admission follows.

The separate review/controller implementation is
`8865efe5aed90d129c349cf350e782b5cb1ccc8c`. This is the helper commit, not the
tested native source or a frozen C74. The [controller documentation](../../../../tools/package-repaired-gnu-074/README.md)
defines the fixed source and consumed per-mode outputs.

## Fresh source and environment review

A new bounded source-only parser capture reaches closure in two rounds:
391 Rust bodies and 26,757 syntax sites. Eight existing bodies and five added
bodies receive a separate manual review; the other 378 complete source/AST rows
stay exact. The review also checks 58 auxiliary inputs and all 155 targets.
The added auxiliary inputs are the six-composition rule file and its text policy;
the inherited inputs already include the actual provider scripts and imports.
The new outer pre/post map includes all tracked `scripts`, closing the earlier
capture omission of `scripts/perf`.

The fixed review-input SHA256 is
`59f8d00f06e3468626b686201957615fa2b1fbb7bfec4020b63bb863aa48ec8a`.
Fresh complete bodies, contextual module edges, actual catalog, inherited GNU
entry-point groups and the explicitly reviewed new controls are checked before
dispatch. This source/effect review is not compiler expansion, a formal
whole-program proof or a substitute for the preservation assertions.

Raw local WSL resolves `/usr/bin/python3` but not Cargo on PATH. The original
absolute Cargo/rustc path selects GNU Rust 1.94.0. The real memory compatibility
fixture successfully invokes Python's unchanged public-API `--check`, including
its nested Cargo metadata command, from Cargo's actual test-child environment.
Provider normalization also passes with its actual imports and synthetic
temporary inputs. No Python alias, PATH edit, script change or semver workload
is introduced to make either fixture pass.

Actual Git configuration, inherited overrides, Cargo-config ancestry, complete
file bytes/modes, refs and the original qualification identity are checked.
Before/after inventories agree for focused tests, the full command, check and
Clippy. This is local nonroot WSL testing, not server enrollment or qualification.

## Native context and resource contract

Initial read-only capacity checks observe about 8 GB free on the physical backing
volume. The prior full command had consumed about 12.5 GB while compiling; a
second complete checkout/history copy followed by a cold-path build would not
provide credible headroom above the unchanged 4 GiB floor.

The preparation method therefore changes explicitly from the initially proposed
new clone to reuse of the separate **nonqualification** native package checkout.
Its old complete tree, HEAD, refs and configuration are verified first. A new
offline bundle/request then advances only that checkout to the fixed source.
All 54,515 tracked files and 404 refs match the new request. The old bundle,
requests, first receipts and committed evidence remain unchanged; the old
qualification/main worktrees are not used. The directory name still contains
`636feb...`, but source identity is determined from the verified current HEAD,
complete tree and request, never from that directory name.

This retains the existing absolute Cargo build context. The four existing ignored
scratch names, their root ownership and absence of symlinks are checked explicitly.
No sources, evidence, cache or worktree are deleted. Reusing the cache is not a
product performance experiment or a portable build-speed claim.

Both test commands retain 780 seconds including compilation, four case threads,
no filter/retry and the physical/native 4 GiB floor. Full-mode compilation reports
17.18 seconds on this warm context. The lowest sampled physical free space during
the full command is 7,088,787,456 bytes; the floor does not fire. Sampling is not
a hard allocation quota or a future capacity reservation.

## Controls and Linux checks

The first portable controller cohort catches a new test defect: its expected
five-argument Clippy suffix accidentally omits `--all-targets`. That refusal and
the exact candidate source are retained. Correcting the assertion does not
change the actual Clippy command. Final controls pass all sixteen cases on Linux;
Windows passes thirteen with three explicit Linux-only skips. They retain the
original real detached-descendant timeout/floor/retirement tests and add fixed
review-body, module/catalog, stale-control and non-dispatch checks.

The separate twelve-target Linux cohort passes 42 cases with zero failures or
ignored cases. It covers all preceding interpreter/composition repairs, PID,
fresh-status and their reader controls. Its complete result is a prerequisite
to the distinct full command, not a replacement for that command's coverage.

GNU `cargo check -p xtask --all-targets` and strict all-feature/all-target Clippy
pass under their own captured commands and postflight. Cargo.lock and the raw
qualification manifest retain their previous SHA256 values. Live check order,
frequency and original 300/60/10 origins/windows are unchanged.

## Full result and precise remaining refusals

The full command retains the original package scope:

```text
/home/artur/.cargo/bin/cargo +1.94.0 test --offline --locked --no-fail-fast --target-dir /home/artur/.cache/hydracache-074-194-check -p xtask --lib --tests -- --test-threads=4
```

Cargo exits naturally with 101 after complete coverage. The two failed cases are:

- `authority_rust_history_074::independent_current_tree_gate_refuses_unapproved_source_and_runtime_drift`;
- `current_tree_input_audit_074::current_tree_input_frontier_is_fully_accounted_without_path_exclusions`.

The first stops at the memory-compatibility source. The complete frontier audit
independently reports all four bodies:

- `crates/xtask/tests/memory_compat_071.rs`;
- `crates/xtask/tests/memory_profile_071.rs`;
- `crates/xtask/tests/support/fresh_status_guard.rs`;
- `crates/xtask/tests/support/pid_path_guard.rs`.

The local repairs pass their behavior and historical-consumer tests, but their
complete source text differs from what the independent global current-tree
frontier admits. This is a remaining source-composition boundary, not a Python
resolution error, exhausted deadline or product crash. The old global pins,
protected paths, original assertions and nine-patch/81-addition guard are not
changed to accommodate it. The next repair must bind these four exact current
bodies to independently named producers before entering the original current-tree
comparison, preserving its reads, canonical path checks and fallback order.
It needs its own positive/mutation/global-audit controls, then fresh GNU review
and a new distinct full command. Do not accept arbitrary hashes or omit paths.

The CI pidfd/subreaper owner proves kernel ECHILD with `retired=true`, no cleanup
errors and the unchanged five-second retirement budget. Postflight succeeds.
These CI cleanup checks do not prove production all-writer retirement, original
delivery, lifecycle completion or reconnect admission.

## Evidence and release boundary

The [663-member packet](local-runs/repaired-gnu-package-20261027/index.json) retains
561 content-addressed objects: the first control refusal and source, source-only
parser provenance, exact review inputs and bodies, transfer/refresh inventories,
all focused/full/check/Clippy streams and receipts, terminal failure attribution
and complete postflight. Index SHA256:
`2e9f7aa54767b01ff5abed454641a2c78b4108e94c02a46197f944dbc13562fa`.
The parser-assessed full result is complete and red. An integrity check of this
packet must not reinterpret a verifier's zero exit as a package pass.

All new per-mode outputs are consumed. Earlier complete/incomplete first results
stay unchanged, and their passing prefixes are not joined to this run. No new
performance hypothesis, server run, service/clock change, release installation,
product workload or expensive qualification occurs. Full package closure,
remaining security, original execution/recovery, trust and release gates remain
open as described in the [closure plan](release-closure-execution-plan-20261026.md).

Documentation and non-promotable local performance-contract check receipts are
retained in the [separate local-check packet](local-runs/repaired-gnu-package-checks-20261027/index.json).
They do not waive either failed global audit or constitute a full milestone pass.
