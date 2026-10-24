# Wider xtask package: first refusals and bounded partial outcome

The narrow registry-reader correction `4f3abd23` passes sixteen focused tests,
`cargo check` and strict Clippy. The separate wider package is **not green**.
The [27-member packet](local-runs/package-frontier-20261024/index.json) retains
25 content-addressed objects, index SHA256
`3ef3c590c843488e0c93e003fc08a5415bd93b9118e3b71e14efdcf52b8b0734`.
It includes both package invocations, reader intermediate refusals, final focused
verification, exact changed source files and the coordinators.

## Narrow correction and controls

The first full affected-package ordinary invocation stops with exit 101 at
`authority_context_074`: its old source-preservation macro receives the current
registry instead of the exact preceding registry view. Both this reader and
the actual registry are byte-identical to the beginning-of-window source
`d9a2b85c`; this specific mismatch predates the parser candidate.

Only the registry path is routed through the already exact-checked historical
input view. All other reads and existing assertion bodies are unchanged. The
independent canary compares the exact reader delta, the unchanged registry and
the old reader/registry to the starting source; unknown edits still refuse.
No runtime admission check is changed.

The first added canary failed module ordering under rustfmt. A separately
corrected placement then failed E0061 due to an incorrectly qualified helper.
Both first logs are retained; the latter rejected source is retained as raw
bytes. A complete raw snapshot of the earlier ordering-only candidate was not
captured. The final explicitly qualified canary passes with the old controls:
authority context 2, historical view 3, immutable cache 7 and owned case 4 tests.
These changed candidates are not retries of the same source to recover a pass.

## Separate changed-package invocation

At commit `4f3abd232801eb44f01b4ffdfc1ffdb91d68a5bc`, the separate command is:

```text
cargo test --offline --locked --no-fail-fast -p xtask --lib --tests -- --test-threads=4
```

It uses an explicit 780-second outer bound and a 4-GiB free-space floor. The
outer timeout fires while package coverage remains incomplete; the wrapper
terminates only its owned Cargo/test process tree and returns failure. Tracked
source hashes, HEAD and tracked cleanliness are unchanged throughout. Timeout
cleanup is not a test pass, completed package or production retirement proof.

The retained stdout contains sixteen failed case lines, spanning authority hex,
current-tree/role census, custody cost hooks, held consuming/build/trust/material
source preservation, document scratch, fresh status and pid-path preservation.
Exact names are in `facts.observed_failed_case_lines` in the packet. These are
observed completed failures, not a complete count of package outcomes: later
tests may be unexecuted or interrupted. No aggregate all-package pass count is
derived. The 321 previously passing performance source contracts do not cover
every source test in this broader package.

Offline log inspection separates inventory-list mismatches (held fixture role
generations) from whole-body mismatches (fresh status/pid-path/custody/census),
scratch-hook counts and registry preservation. This is an assertion-location
classification, not root-cause proof or permission to normalize those inputs.
Several tests intentionally catch panics from mutated counterexamples; individual
panic lines are therefore not additional failed cases. Only final `FAILED` case
lines supply the sixteen observations above.

One inventory assertion has an additional read-only baseline proof. The test
`actual_consuming_fixture_preserves_all_446_preceding_sources_and_formats`
directly compares the 446-path `6c6e47ee` source tree to raw current `git ls-files`.
Its test file, including the unadapted `git` helper, has the same Git blob
`e0ac74524bbb1f43624a6204dea8a7ac0ffd9b28` at the start `d9a2b85c` and final
documentation source `387d1e85`. Immutable `git ls-tree` counts are 446 at
`6c6e47ee`, 528 at `d9a2b85c`, and 530 at `387d1e85`. Thus that direct inventory
equality already conflicts with the starting tree; the candidate's two added
parser-control source files are not its sole cause. This is static source/Git
inspection, **not an executed baseline test**. Removing only those two filenames
would still leave 528 unequal to 446. No source-list filtering or assertion
change is applied. It does not attribute the remaining fifteen failed cases.

Do not infer a common historical cause for all sixteen failures from the one
registry diagnosis. Next inspect each retained first assertion against its exact
reviewed predecessor, establish baseline provenance without live reruns, and
add independent rejection controls before changing a reader. Do not broaden
source normalization, drop assertions, filter away red tests, raise the outer
test budget to conceal the result, or repeat the unchanged command to pass.

This package result neither reruns nor repairs the first full milestone's
[dependency-policy refusal](dependency-security-plan-20261024.md). Runtime
delivery, all-writer retirement, qualification and release admission remain
separate and closed.
