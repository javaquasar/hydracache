# Current tree history compatibility and independent source checks

The independent current-tree history gate now passes with its original ten
tests, historical baseline, thirty guard definitions and seventy-nine assertion
test names retained. The final focused cohort has thirty-six passes. This is
test-source compatibility, not production enrollment or execution admission.

## Complete frontier rather than the first mismatch

The original [package frontier](package-frontier-result-20261024.md) first
reported `authority_context_074.rs`. A separate read-only collector finds ninety
positions after applying the existing verified input gateway: nine changed
historical reader/normalizer files and eighty-one source/test files added after
the fixed historical baseline. Its first refusal and complete path list remain
retained; it is not replaced by a new baseline that hides those differences.

The fixed rules in `crates/xtask/tests/support/current_tree_input_rules_074.json`
bind source `33c54b0cda8c330b115009cdf092ad002077cedf` and declared current
snapshot `b571bef2bcd2ceb3ed044c9e31bce4f670cfafdd`. Their canonical SHA256 is
`85ab712a7b5e11d5430749108df9a013887eb027f8d9ce8683b4676f1b5f83fa`.
They explicitly state `test-source-compatibility-only` and
`execution_admitted=false`.

For each of the nine old files, literal context patches reconstruct the current
file from its frozen predecessor. Every before-fragment must occur exactly once;
both input/output digests and the entire declared current file must agree.
Only then is the old version exposed to the historical assertion. Seven rules
cover reader wiring already checked by their independent preservation canaries.
Two cover existing test-only service/owned-case hooks and the exact ordering
needed to restore the bounded projection parser before its older prefix view.
No runtime validation body or historical assertion is excluded.

The eighty-one declared additions require exact path membership, regular
non-symlink files, no Windows reparse point, exact tracked mode/blob/stage and
full canonical bytes/digest/length. Unknown additions still hit the original
`previous.contains(path)` refusal. The source catalogue is not an assertion that
those components can authenticate original execution or stop all writers.

## Scope correction before acceptance

The first reader canary has two passes and one expected missing-wiring failure.
The first integrated candidate reveals that the older literal gateway emits an
unformatted wrapper, whereas its checked-in predecessor is rustfmt-formatted.
The extension recognizes exactly those two wrapper forms; unknown forms refuse.
That first failure remains recorded.

A general gateway extension then passes twenty-four focused cases and fifty-four
cases spanning all previously failing targets. Its strict Clippy invocation
detects a duplicate module import. More importantly, the direct historical-cache
control refuses the changed common gateway: six of its cases pass and its
complete-loader-preservation case fails. That global-gateway candidate is
rejected, not treated as an acceptable partial install.

The accepted design leaves `tools/authority-runtime-074/gate.rs` unchanged from
the preceding commit. Compatibility is scoped to the current-tree history reader
and the single history-file input inspected by `authority_runtime_074.rs`.
A shared literal extension reconstructs the complete allowed current history
test from its pinned predecessor, retaining every old assertion. Other runtime
contract reads keep their original IO. The new canary also reconstructs the
entire permitted runtime-test reader delta. No duplicate import or lint waiver
remains.

Accepted code commit `a329cac91784d79d9e5b925c22d882cb0d327283` changes seven files:

- `crates/xtask/tests/authority_rust_history_074.rs`
- `crates/xtask/tests/authority_runtime_074.rs`
- `crates/xtask/tests/current_tree_input_audit_074.rs`
- `crates/xtask/tests/current_tree_reader_074.rs`
- `crates/xtask/tests/support/current_tree_input.rs`
- `crates/xtask/tests/support/current_tree_input_rules_074.json`
- `tools/authority-runtime-074/history_reader.rs`

## Focused checks and refusal controls

All ten original history cases pass. Together with the runtime contract 5,
complete frontier audit 1, new canary 4, reviewed inputs 3, immutable cache 7,
owned-case controls 4 and status-field controls 2, the final source has thirty-six
passes across eight targets, with none ignored or filtered. All-targets check,
strict all-targets/all-features Clippy and six-file rustfmt pass.

Controls reject altered assertions and safety predicates, broad source selection,
missing new-source validation, empty/extra/changed old inputs, unknown source
names, altered new-source bytes/comments, mode/blob/stage changes and duplicate
index entries. The original unknown-command/path, current-candidate, qualification
identity and historical-IO controls remain. Only immutable rules/source blobs
are cached; live filesystem/index observations for declared sources are fresh.

The [retained packet](local-runs/current-tree-history-20261024/index.json) keeps
first frontier/canary refusals, formatter and Clippy findings, the rejected common
gateway, final scoped source and raw commands with before/after source identities.
It contains 103 logical members in 80 content-addressed objects; index SHA256 is
`e93e5fcfe733d5e0dfca90660f1b98a2a749775f431de9886b1cad309cb8b23f`.

## First complete package outcome

The changed-source Windows MSVC `cargo +1.94.0 test --offline --locked
--no-fail-fast -p xtask --lib --tests -- --test-threads=4` finishes naturally
inside the unchanged 780-second outer bound. Across 128 target verdicts,
1173 cases pass and three fail, with none ignored or filtered. Exit is 101;
there is no outer timeout and both source and HEAD remain unchanged. No retry
is made. The original partial sixteen-failure/timeout result stays immutable.

The newly reached refusals are:

- `remaining_preparation_cost_074`:
  `bounded_projection_recovers_all_473_preceding_bodies_and_frozen_parsers`,
  at the diagnostic held-launcher source comparison.
- `validation_leaf_requests_074`:
  `validation_leaf_requests_only_add_exact_test_hook_and_registered_manual_gate`,
  at the historical explicit-gate registry comparison after its negative controls.
- `validation_round_cost_074`:
  `round_observer_recovers_all_477_prior_bodies_without_new_runtime_path`,
  at the diagnostic held-launcher source comparison.

These need separate exact-reader attribution and mutation controls. The whole
package remains red despite the accepted focused history repair. Documentation,
local non-promotable performance-contract and 188 registered-gate checks pass;
they do not override the three package refusals or the dependency milestone.

## Unchanged release authority

Supervisor/product code, Cargo inputs, `deny.toml`, ReservedLive observation
frequency/order and original deadlines remain unchanged. Qualification manifest
SHA256 stays `11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
The fifteen-advisory dependency refusal remains audit-only. No server action,
clock/service change, original-owner rerun, product measurement, expensive
qualification, additional cleanup or push occurs. Preparation fit, actual
delivery/enrollment, all-writer retirement, durable lifecycle/cells and crash/
reconnect proof still block server admission in the
[closure audit](release-closure-audit-20261024.md).
