# Complete native package result after capacity repair

The separately admitted GNU command completes all 153 targets within the
unchanged 780-second limit: 1,324 cases pass, ten fail and none are ignored.
The gate is red because of test failures, not incomplete coverage, timeout or
disk refusal. Full native and Windows source/ref/HEAD postflight passes. The
previous Windows V9 timeout and both native compilation refusals remain intact.

Controller implementation: `160dbe09bc4bbe80d847761d23a1ce2bf71ab10f`.
Actual native package source: `636feb166b273b5c3d6fc0b93c4fccf57560cf03`.
The controller change selects a new exclusive output/control receipt only; its
command, helpers, predicates, ownership and deadlines remain unchanged.

## Capacity and execution

The operator removes the previously identified old Windows PDB symbols. The
read-only preflight then observes approximately 24 GiB physical free space,
no active Linux compiler and no unexpected native in-tree target. No source,
executable, worktree or evidence is deleted by the agent in this step. The
package capture records 25,734,336,512 physical free bytes before execution and
13,178,982,400 after execution. These are observations, not a disk reservation.
The unchanged 4 GiB physical/native floor does not fire.

The full command remains:

```text
/home/artur/.cargo/bin/cargo +1.94.0 test --offline --locked --no-fail-fast
  --target-dir /home/artur/.cache/hydracache-074-194-check
  -p xtask --lib --tests -- --test-threads=4
```

Fresh source/effect/environment checks precede the command. Cargo prints
229 seconds for compilation. The strict, unchanged transcript parser accounts
for all 153 scheduled and completed targets, without an incomplete target or
unstarted suffix. Their printed spans total 307.38 seconds; this sum is not an
independent CPU or end-to-end measurement and supplies no product speedup claim.

The original Linux CI group records natural Cargo exit 101, `test-failure`,
`retired=true`, no cleanup errors and the same five-second cleanup budget.
Kernel ECHILD precedes the complete native tree/ref/config/catalog postflight.
The outer capture reports no refusal and unchanged Windows source and HEAD.
There is one package dispatch and no automatic retry, suffix join or Windows
replay. The 780-second bound includes compilation; test concurrency stays four.

## Failed cases and repair boundaries

| Target | Passed | Failed | First observed refusal |
| --- | ---: | ---: | --- |
| historical_source_cache_074 | 5 | 2 | Exact expected/current token comparison |
| memory_compat_071 | 4 | 1 | Required `python` executable not found |
| memory_profile_071 | 4 | 1 | Required `python` executable not found |
| pid_path_cost_074 | 2 | 1 | Kernel-origin predecessor body differs |
| remaining_readers_074 | 1 | 3 | Whole-caller/current-source and pinned-successor assertions |
| role_census_reader_074 | 1 | 1 | Reconstructed caller differs from current caller |
| validation_leaf_reader_074 | 1 | 1 | Current registry differs from frozen baseline |

The two executable failures are direct `Command::new("python")` calls in the
0.71 public-API inventory and provider-contract fixtures. The earlier GNU
preflight checked `python3` but missed these two additional `python` prerequisites.
Its successful review receipt is preserved, but does not prove this environment
was sufficient for every default test. Before another candidate, review these
specific script bodies and their complete subprocess/import effects, then choose
and test an explicit cross-platform interpreter contract. Do not install an alias,
change PATH during a run, skip the tests or accept the missing tool silently.

The other eight failures are exact source/predecessor consistency assertions.
The retained logs identify the first refusing equality or pin, not a completed
proof that every later assertion would pass. Audit each actual whole-body delta
and existing composition before constructing any narrow predecessor view. Keep
the independent current-input pins, read order, mutation/refusal cases and every
original assertion. Broad whitespace normalization, a new expected digest alone
or returning an arbitrary historical body would not establish compatibility.
No failed assertion is removed and no repair is claimed in this execution step.

## Controls and evidence

The added contract control initially fails on Windows because `Path` renders a
native path with backslashes. That first result and its exact source are retained.
The corrected test separately pins the POSIX target path and compares its native
rendering in the command. No dispatcher or GNU command changes for that repair.
Final controls pass ten Linux cases and seven Windows cases with three Linux-only
skips. Source hashes bind the required green Linux control receipt before Cargo.

The [81-member evidence packet](local-runs/native-full-capacity-restored-20261027/index.json)
retains 42 content-addressed objects: all first/final controls and source witnesses,
both Cargo streams, the full review, controller receipt, strict accounting,
postflight and a bounded failure index. Index SHA256:
`e9ca6dd7620d908055ad4c6cc36e739b168a473cdf2380224a8ada9de4ac6564`.
Archived source remains data only. The [earlier resource result](native-full-package-controller-result-20261027.md)
keeps its missing final receipt and original classification.

Doc-check and the local non-promotable performance-contract check pass. Independent
data verification reproduces the strict accounting exactly, rejects four tampered
transcript/exit/bound/catalog variants and verifies all 43 staged packet blobs,
including every early source witness. Cargo.lock and the qualification manifest
retain their prior raw hashes. The [37-member local-check packet](local-runs/native-full-restored-checks-20261027/index.json)
retains those outputs and the verifier source; its index SHA256 is
`eb2debdd27139f2daa9f48fcfd79a7ec731d0d32f0a65d017d7b8be7e09b2af7`.
Successful receipt verification establishes the integrity of a red result, not
a package pass.

## Next gate

Repair the interpreter contract and the eight exact composition failures under
separate focused controls, review the complete changed source/effect/environment
closure, and admit a distinct full command only after those controls pass.
The current output directory is consumed. Do not repeat this unchanged candidate
or combine its passing cases with other attempts. Required Windows coverage and
the [other release work packages](release-closure-execution-plan-20261026.md)
remain open. No new hypothesis, service/clock/server operation, release install,
product load or qualification occurs. Cargo.lock, qualification identity and
all live checks and original 300/60/10 origins/windows remain unchanged.
