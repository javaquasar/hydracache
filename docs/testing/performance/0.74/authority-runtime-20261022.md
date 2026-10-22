# Owned storage runtime and native filesystem correctness

The exact owned-storage candidate is installed in the supervisor authority
engine and passes native Linux compilation and filesystem checks on Rust 1.94
and MSRV 1.88. This closes the local integration boundary from the
[historical Rust routing work](authority-rust-history-20261022.md), not original
start, full process-loss recovery or server admission.

Preregistration: `acd44d86daa6d55490a2565c9d0209f5775922a4`.
Method: `37db65f68e67ed7c0ee0f1cc84fed1c30bf9822a`.
Evidence: `08f0437623739fbcee4d42e476d90c444999c124`.
The [plan](authority-runtime-plan-20261022.md), [derived facts](local-runs/authority-runtime-20261022/facts.json)
and [raw packet](local-runs/authority-runtime-20261022/) identify their separate
phases. No old measurement image was executed.

## Exact runtime installation

[diagnostic_authority_fixture.rs](../../../../tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs)
and the [mutable owner observer](../../../../tools/long-run-supervisor-074/tests/support/authority_owners_controls.rs)
match the complete candidate and observer token streams pinned at
`f193fabae86c9bd7a2e778a9463a43984834deb5`. The frozen artifacts themselves
remain unchanged. Five legacy test helpers receive only required scratch None
initialization or mutable receiver/caller adjustments. Their assertions remain;
old ignored allocation probes compile but do not execute.

Each Leaf owns an optional scratch buffer. A fresh open, metadata checks,
bounded read through EOF and original stamp/byte comparisons still run in their
original order. Reuse selects only matching capacity and clears the previous
length. Successful comparisons may retain storage only when actual capacity
equals observed file length and stays within the original budget. The fresh
File remains held through the comparisons; original File, stamp and raw
authority bytes are not replaced.

External refusal, validation, binding, transition and publication failures all
evict the whole journal's scratch while preserving authority objects and history.
Record-before-marker order, publication fsync/checkpoint boundaries and final
closing validation are unchanged. Reopen does not transfer an old owner's
scratch; its mandatory final validation may warm new slots.

## Current validation is separate from historical assertions

The new [raw-current gate](../../../../crates/xtask/tests/authority_runtime_074.rs)
checks complete installed files against independently derived frozen
expectations. It rejects baseline-only installation, partial append, extra
effects, removed checks, changed cleanup and altered assertions before returning
any predecessor view. Unknown paths retain current bytes and remain subject to
their original guards.

The older eight storage-source and ten Rust-history controls retain their
complete test bodies and assertions as historical phase checks. The full
performance-contract binary needed an additional transitive IO boundary:
fifty support modules each change exactly one actual read call, plus explicit
imports in the root binary. Literal restoration templates and assertions do
not change. Complete-file equality checks independently guard these adapters.
Only verified approved current files receive the fixed predecessor before old
normalization. The root binary's other direct current-source reads remain
unchanged.

All 321 original performance-contract tests remain selected. Success of a
historical view alone cannot admit the installed runtime.

## Actual filesystem coverage

Seven new tests are appended to
[the original fixture tests](../../../../tools/long-run-supervisor-074/src/diagnostic_authority_fixture_tests.rs).
They cover cold construction and poisoned scratch, successful fresh reads,
capacity bounds, ten leaf refusal variants, original descriptor/stamp/byte
retention, all five journal cleanup boundaries, successful epoch transitions,
reopen and no cache transfer. A closing-publication refusal checks eviction of
both old and newly appended entries while retaining the journal.

Four logical publication cuts check sticky failure, partial-tail refusal without
repair and a committed refusal visible after reopen. These are injected error
boundaries, not real SIGKILL or lost-ACK delivery tests.

The frozen [selection](../../../../tools/authority-runtime-074/tests.json) contains
33 tests: seven new and 26 existing fixture, fixed-backend and owner/leaf/breakdown
controls. Existing controls retain coverage of replaced or missing files,
ownership/mode changes, links, FIFO and stale/mutated bytes. Each selected test
runs separately with --exact and --test-threads=1; none is ignored. All filesystem
effects stay in private temporary fixtures. Changing a fixture file's timestamp
is not changing the system clock.

## Frozen final results

Both native lanes use method `37db65f6`, nonroot UID/GID 1000/1000, WSL kernel
6.6.87.2-microsoft-standard-WSL2 and the same final boot
`86da81ea-17c5-4ccf-ba96-0ff1f074c08f`.

| Lane | Compilation and lint | Filesystem tests | Test image SHA-256 |
| --- | --- | --- | --- |
| Linux Rust 1.94.0 | all targets/features check and strict clippy pass | 33 passed, zero ignored | `0f941071b683fc78724ec8d329352243b3a2da0ab45e4c2e3f4bb435cf40cee6` |
| Linux Rust 1.88.0 | all targets/features check and strict clippy pass | 33 passed, zero ignored | `c6fa8b7939659ce0f00a5164eb1d033c6216da38b6b88c7bf49278754cddcb10` |

Each lane has 38 recorded steps, stable executable bytes and identical pre/post
source, boot, credentials, configuration hashes and qualification metadata.
Separate newly owned targets avoid changing old pinned profile/qualification
images. Development and final Rust 1.94 use identical test-image bytes; this is
not a new performance measurement. These are debug-profile correctness builds,
not release performance images.

The first frozen Windows cohort passed all nineteen steps: five new current
source controls, eighteen prior source controls, 79 historical canaries in
thirty binaries, all 321 performance-contract tests, eight selector/packet
controls and twenty Python history/receipt controls. Format, supervisor and
xtask checks/strict clippy, doc-check, registration of 186 discovered gates and
the local non-promotable performance-contract CLI pass. Registration does not
mean all 186 gates executed, and this is not a full release verify.

## First failures remain distinguishable

The pre-install Linux Rust 1.94 compile baseline passed. Development logs retain
the incomplete seventh-test append, selector count correction, missing rustc
PATH and duplicate-module lint refusal. None is relabelled as a successful first
cohort. The first driver failed before native compilation; the repaired driver
used a different output directory. Its first executed native development
selection passed all 33 tests.

Full Windows development cohorts first passed 290 with 31 failures, then 306
with 15 failures. These were historical reader boundaries, not native filesystem
failures. The focused dependency cohort passed 234 with one custody_join
preservation failure; static source-reference audit completed its four-module
chain. The repaired focused cohort passed 235, with 86 explicitly filtered for
that development diagnosis. The frozen final cohort runs all 321 without those
filters.

Exact import-order and packet path negative controls also exposed harness errors;
their corrections and the available raw output are described in the retained
[development notes](../../../../tools/authority-runtime-074/development-refusals.txt).
Some initial diagnostics exist only in chat, as those notes explicitly state.

The native development boot was
`628ddbd4-3645-4c12-8d5a-4bd3bf56cedc`, distinct from the final boot. Each
cohort's own before/after pair is stable, and both final lanes share a boot.
No manual VM restart, clock or service change was performed. Do not treat these
separate WSL sessions as one timing cohort.

## Evidence verification and remaining boundary

The inventory seals 364 members, excluding inventory.json, across 365 files.
It includes 75 committed-source snapshots, source.zip, raw baseline/development
logs, first driver refusal, development native output and all final logs.
Inventory SHA-256:
`fafe2b63fde717f1f34d3ad62b06a50014c9653916a8a8b79577fd9f91568375`.
All 365 working packet files matched their staged raw Git objects before the
evidence commit. The prior inventory and its 61 members remain byte-identical.

Offline verification, without executing a native image:

```text
python -B tools/authority-runtime-074/review.py docs/testing/performance/0.74/local-runs/authority-runtime-20261022
```

The native runner is a bounded local repository/toolchain harness, not a portable
server enrollment mechanism. Its final mode requires clean committed tracked
sources; existing output directories refuse overwrite.

This correctness result does not measure allocations/op, CPU/op, latency, copied
bytes or RSS/retained-memory improvement. Retaining scratch can trade fewer
allocations for more live memory per Leaf; bounds and eviction do not establish
that tradeoff's performance value. A separately pinned current-runtime
attribution must measure it without reusing old model results.

Original validation checks/count/order, 300/60/10 windows, first Causal refusals
10042390247/10010835739 ns and qualification raw digest
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`
remain unchanged. No live/original-owner preparation, authenticated start,
real process-loss campaign, server action, installation, supervisor action or
product workload ran. Full SIGKILL/lost-ACK, all-writer revocation, reconnect,
later lifecycle cells and real trust/enrollment gates remain separate.

## Documentation follow-up

The documentation follow-up passes doc-check, the 186-gate registration check,
the local non-promotable performance contract, all 23 current/historical source
controls and offline verification of the sealed packet. Its first parallel
Cargo source-control invocation failed before tests with os error 5 removing
xtask.exe while the parallel CLI checks were running.
The first raw log remains separate; the serial invocation passes without source
or assertion changes. No process/service is stopped and no native/live cohort
is rerun for this tooling repair. Follow-up receipts are retained
[separately](local-runs/authority-runtime-doc-followup-20261022/).
