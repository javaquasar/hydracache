# PID historical reader stage repair

The isolated PID origin composition passes the focused Windows 1.94 and MSRV
1.88 cohorts. Each complete command runs twelve targets with 42 passes, zero
failures and zero ignored cases. Implementation:
`7f4d185820e0c14a9fba4bb482f98e65df5172ce`. GNU validation and the full package
gate remain open; the retained full GNU result is still 1,324 passes and ten
failures on its earlier source, not a result for this repair.

## Actual reader boundaries

The first source-only probe preserves the original conditional reader: admission
uses `runtime_io`, while origin and the allocation-hook file use the filesystem
reader. Its before/after source maps match. No live host fixture executes.

| Path | Reader producer | Stage already equal to PID producer |
| --- | --- | --- |
| origin | `46648903c0eb7ce88d438008fa6465a96cc3c414` | None before the new join |
| admission | `33c54b0cda8c330b115009cdf092ad002077cedf` | After document normalization |
| allocation hooks | `46648903c0eb7ce88d438008fa6465a96cc3c414` | After custody normalization |

The named PID producer is `b5f1b8fce6b3046239087873cb85ff1bfc557f0f`.
Origin alone retains later FD-identity substitutions and operation-observer
hooks. Its post-custody SHA256 is
`4eb7a1463e4d2318ecdfee44759aafb632c950da4a9309f39d26004176a59c8a`;
the complete PID-producer SHA256 is
`05cfe2ccbad64af3aaa4c38f65535ff3039e5051c0812e99502b79e2933c8bcc`.
Both bodies and all three reader producers are independently checked against
named Git objects. The earlier raw-admission adapter was unnecessary at this
boundary; accepting another hash would not establish the correct join.

## One path and an unchanged legacy consumer

`crates/xtask/tests/support/pid_stage_join.rs` applies fifteen exact literal
inverse edits only to origin, after the existing custody view and before the
original PID substitutions. Each requires its exact count. Other paths pass
through unchanged. The helper performs no IO, Git operation, current-observation
cache or historical-code execution; it is a source-test composition, not a live
validation or execution permit.

Both original PID and fresh-kernel test files remain byte-for-byte equal in
their original text representation. PID's hook/formatter/change loop remains
exact. Its complete final-body comparison against
`b63a0e81ad808d6f65e8c8d818ee4c090c3718ff` remains mandatory.
Fresh-status already enters an older origin stage, so its only change is an
explicit call to `normalize_legacy`, retaining the old custody/PID processing.
Applying the new join indiscriminately to that consumer would be wrong.

The stage digests are provenance controls in the new test, not a blanket hash
gate replacing the old normalizer. A semantic mutant must still reach PID's
original `assert_ne!` checks instead of necessarily panicking early. Bytes
outside the fifteen exact inverse hunks survive to the old whole-body comparison.
New controls exercise missing/duplicated hunks, stale producer input, 64 seeded
mutations, unrelated appended text, success after refusal and nonselected paths.
They also verify the old PID loop, both unchanged readers and the exact single
fresh-status call substitution. Existing reader and immutable nine-patch/
81-addition guards pass without changing their pins.

## First results and focused checks

The first candidate cannot compile because the locked SHA256 array lacks
`LowerHex`; its exact source and compiler refusal are retained. Byte-wise hex
encoding fixes only the diagnostic formatter.

The next command completes seven targets with eighteen passes and one failure.
The new legacy control incorrectly supplies the already post-custody allocation
producer to a function that must still run custody processing. The existing
custody hook assertion correctly refuses the missing hook. The corrected control
starts at the original document boundary; no hook check becomes optional.
That command's exact candidate source and first result also remain archived.

The repaired seven-target command passes nineteen cases. Subsequent composed
Windows and MSRV commands each pass 42 cases across twelve targets, covering
the prior interpreter/source repairs, PID, fresh-status and their reader guards.
These are separate complete focused results, not joined package coverage.

```powershell
cargo +1.94.0 test --offline --locked --no-fail-fast -p xtask --test pid_stage_join_074 --test pid_path_cost_074 --test fresh_kernel_cost_074 --test remaining_readers_074 --test package_closure_inputs_074 --test pid_path_reader_074 --test fresh_status_reader_074 --test memory_compat_071 --test memory_profile_071 --test historical_source_cache_074 --test role_census_reader_074 --test validation_leaf_reader_074 -- --test-threads=4
```

The MSRV command changes only the toolchain to `+1.88.0`. All-targets check,
strict all-feature Clippy and targeted rustfmt of all four changed Rust files
pass. No new whole-workspace formatting pass is claimed; the earlier Windows
command-length refusal remains retained.

## Evidence and remaining admission

The [133-member packet](local-runs/pid-stage-join-20261027/index.json) retains
65 content-addressed objects, nine capture phases, both rejected candidate
snapshots, producer bodies and final source witnesses. Index SHA256:
`ab803300712ce26ac4e9439bf8e3ba50c32c2de8b0788d580aad86f86c3e4c2b`.
The first stage probe has pre/post source hashes and its complete output, but
no separate initial probe-source snapshot. The reused outer capture excludes
`scripts/perf`; provider-script witnesses match the named earlier Git bodies
after execution, not a temporal pre/post inventory. A new GNU review must cover
those scripts and their actual test-child interpreter/Cargo resolution.

Next perform that fresh source/effect/environment review, validate the repaired
cohort on GNU, and admit a distinct full package command with the unchanged
780-second compile-inclusive bound, four threads and physical/native 4 GiB floor.
The two newly added ordinary targets change the future catalog; the old fixed
153-target review cannot admit this source. Do not replay unchanged V6/V7/V8/V9
or join an old prefix to new suffix results.

This repair changes no product path, dependency, qualification identity, live
check frequency/order or original 300/60/10 origins/windows. There is no new
performance measurement or improvement claim, server/service/clock action,
product workload, installation, qualification or release admission.

Doc-check and the local non-promotable performance-contract check pass. Their
[separate local-check packet](local-runs/pid-stage-join-checks-20261027/index.json)
retains the command receipts; neither check substitutes for the GNU package gate.
