# Full Windows package closure attempt

The operator authorizes one full package gate on the reviewed current source,
preserving checks, deadlines and first results, without opening new hypotheses.
This is a correctness/coverage run, not performance qualification or production
execution. The old partial/red package results remain unchanged.

## Fixed command and bounds

```text
cargo +1.94.0 test --offline --locked --no-fail-fast -p xtask --lib --tests -- --test-threads=4
```

Run the command once through the original 780-second wrapper, including Cargo
compilation in its original timing origin. Keep four case threads,
`CARGO_INCREMENTAL=0`, `CARGO_BUILD_JOBS=2` and the original 4-GiB disk floor.
No filter, suffix-only execution, `--ignored`, changed target order, telemetry
observer, prebuilt-test bypass, retry or enlarged bound is permitted. Readiness,
dispatch and partial verdicts cannot be turned into a complete package pass.

Exact original wrapper SHA256 values:

- `target/run-xtask-package-780-20261024.py`:
  `52504f2b489d16f344b477df0568f33ac224c5cadacc552f32f427f39c09d04d`.
- `target/run-source-step-20261024.py`:
  `a92482cabcfcf8f6440e95972bee8d204bb9c59c8805c45c7b57363dcc3ab430`.

## Independent source and environment review

The [v4 source-only packet](local-runs/package-effect-syntax-v4-closure-20261025/README.md)
has 344 Rust sources, 25144 sites and all 138 default targets. Compared with v3,
only the read-only GET-owner terminal test is added and its retired overlay
projection changes in the old performance-contract target. The other 342
source/AST rows and all 29 auxiliary pins are identical. No new subprocess,
service, host or clock entry point is introduced by those two changes.

`package_effects_review_v4.py` separately pins that exact graph. V1–v3 are not
edited or taught to accept it. Existing environment/build/Cargo-config/negative
executable and Git effect guards are inherited unchanged. Inherited fixture,
capture and test filters refuse rather than being removed from the environment.
Untracked or dirty source, moved module edges, source/AST/auxiliary drift and
wrapper drift refuse before the package starts. The source review is manual
default-Windows clearance, not arbitrary macro expansion or a sandbox proof.

`package_replay_v4.py` writes an exclusive intent before preflight. It defaults
to no dispatch and accepts no configurable command, target, threads or deadline;
`--execute` invokes the unchanged wrapper once from the clean committed tree.
The separate fixed-target observer dispatch receipt remains closed and unchanged;
this run does not use that attribution path or claim its integration is finished.

## Required retained outcome

Preserve syntax/build inputs, focused review controls, explicit dispatch intent,
clean source/HEAD before and after, wrapper hashes, complete raw stdout/stderr,
cleanup log if present and the wrapper's first exit/outer-refusal record.
Independently reconstruct all 138 targets and case verdicts from that same
transcript. Distinguish completed targets, a started but incomplete target,
individual case lines without a target summary and targets never started.

A package pass requires natural zero exit, no outer refusal, unchanged source
and complete actual coverage. A timeout, failed case, absent target verdict or
cleanup uncertainty stays red. Focused passes from earlier runs cannot fill a
missing verdict. Preserve and diagnose the first new failure before deciding any
repair; do not run the unchanged attempt again to obtain a different outcome.

The attempt grants no dependency update or MSRV override. The fifteen-advisory
milestone remains separate and audit-only. Frozen 0.73 artifacts, qualification
manifest, product workloads, existing services, clocks and server are unchanged.
Original execution, all-writer retirement, later cells, reconnect and production
enrollment still require their own gates even if this Windows package passes.
