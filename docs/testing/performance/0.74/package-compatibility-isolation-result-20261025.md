# Frozen source compatibility repair

All seven source-compatibility failures from the first v2 package replay now
pass in a separate focused check on Rust 1.94 and MSRV 1.88. The repair restores
four frozen sources instead of widening their guards. The original 780-second
package timeout and first seven failures remain retained; the package is not
green and is not rerun here.

Code commit: `7b0059d43fcca65afa37500f0eb0638deffbc20a`.

## Baseline and repair decision

The [first v2 result](package-effect-review-v2-result-20261025.md) identifies two
integration mistakes. Additive diagnostics changed complete test-source bodies
that older guards intentionally pin. The small path-comparison candidate also
changed a fixed-slice reader and the runtime gate expected by older contracts.
Passing the individual diagnostic target did not establish that those edits
were compatible with the full package.

The repair restores these four files to their independently retained
`3e9bcc06ebce3439495200162cb0c1900b8eda53` versions, comparing complete text with
CRLF normalized to LF:

- `crates/xtask/tests/authority_context_074.rs`;
- `crates/xtask/tests/diagnostic_artifact_fixture_074.rs`;
- `crates/xtask/tests/support/performance_contract_074_fixed_slice.rs`;
- `tools/authority-runtime-074/gate.rs`.

The two original authority-context tests remain in their old target. All sixteen
added diagnostic tests move into the ordinary `package_diagnostics_074` target:
thirteen bodies are unchanged, one changes only the module alias, and two now
assert that observers/the path candidate are not installed. Missing/duplicate
hooks, assertion/body drift, unknown paths, frozen-output checks, default-off
behavior and pure buffer controls remain. No test is ignored or discarded.
Default integration discovery consequently grows from 136 to 137 package
targets; this is an inventory change, not a successful full-package verdict.

The artifact helper again contains its exact original read/Git/output/decode
body, with all twenty assertions/cases intact. Historical helper attribution
now erases the sealed measured source, while separately requiring the installed
helper to equal its original baseline. Its capture runner still refuses the
uninstrumented source; it does not synthesize a replacement profile or silently
retarget the old experiment. Normalization/helper capture recipes tied to the
old target/build receipts remain historical, not new runnable protocols.

The [path allocation result](guard-path-comparison-result-20261025.md) remains
valid for its isolated scope. Its installed predicate and gate adapter are
withdrawn because complete-source compatibility fails, not because the earlier
allocation counts were disproved. The standalone contract/probe, first samples
and mutation controls survive. No full-path benefit or product saving is claimed.

## Preserve the reviewed scope rather than refresh its pins

The v1 and v2 production reviewers and their literal admission pins are
unchanged. V2 must reject the repaired tree. Its positive unit tests now use the
exact archived v2 sources/auxiliary bytes in a temporary data-only tree; no
retained Rust source is executed. The target catalog is explicitly substituted
for this unit-level fixture, not represented as a fresh whole-package audit.
A separate negative test checks the actual current tree still refuses.

The offline reuse audit joins the same nineteen saved trace frames and 3157 Git
calls as before. A new changed-installed-body control refuses even with a valid
saved trace. No Git measurement, CPU/allocation cohort, caching or process-capture
candidate is repeated or installed.

## Focused verification

Each compiler runs these nine complete targets with four case threads:
`artifact_inventory_reader_074`, `authority_context_074`, `authority_runtime_074`,
`authority_rust_history_074`, `current_tree_input_audit_074`,
`current_tree_reader_074`, `historical_source_cache_074`, `owned_case_074` and
`package_diagnostics_074`. There are no within-target filters.

| Check | First outcome for this repair |
| --- | --- |
| Rust 1.94 focused targets | 54 passed, zero failed/ignored |
| Rust 1.88 focused targets | Same 54 cases passed, zero failed/ignored |
| Restored artifact target, Rust 1.94 | All 20 cases passed, zero failed/ignored |
| Historical-tool Python controls | 82 passed |
| Attribution Python controls | 54 passed |
| All-target check, both compilers | Passed |
| All-target/all-feature Clippy, both compilers | Passed with warnings denied |
| Focused Rust formatting and Python compilation | Passed |
| Python indentation check | Passed |
| Documentation consistency | Passed |
| Local 0.74 performance contract | Passed, non-promotable only |

These checks exercise every previously failing case, old complete-source and
assertion-mutation guards, all nine exact current-input patches and all eighty-one
declared additions. The latter's original byte/index/mode/path checks are not
extended to accept the diagnostic edits; the offending edits were removed.
Every check retains unchanged source/HEAD receipts. This is Windows/default
test execution plus all-feature linting, not Linux live delivery or a full
all-feature test run.

## Evidence and next gate

The [packet](local-runs/package-compatibility-isolation-20261025/index.json)
retains changed/restored sources, migration accounting, eleven check phases,
raw outputs, exact wrappers and protected input hashes. Index SHA256:
`a1d7b2450970748b10f47660938e399b3601965e043911159816565c17325086`.
Independent staged readback verifies all 78 members and recomputes exact focused
case coverage without another producer.

The [documentation checks](local-runs/package-compatibility-isolation-doc-gates-20261025/index.json)
separately retain their raw structural-check receipts, including Python
indentation. Their index SHA256 is
`a173d9001444eab1e4657e7f5182b2779e1102be1a66cecb21913506dde3ad5c`.

Two evidence-coordinator refusals also remain with their source: an incorrect
estimate of unchanged-versus-alias-only bodies, then a catalog missing five
nested cache-module tests. Saved-source/log review corrects those accounting
errors to thirteen unchanged bodies, one alias-only body and all 54 cases.
Neither correction reruns a test or relabels a failed producer.

Next review the repaired source graph independently before another full replay,
and use the existing saved evidence to locate the unfinished contract target's
cost. Do not reuse the stale v2 receipt, splice these focused passes into the
old package result, increase 780 seconds or reduce live observations. Fifteen
dependency advisories remain audit-only. C74 composition, original delivery,
all-writer retirement, lifecycle/reconnect and real enrollment remain open in
the [six-step continuation](release-six-step-continuation-20261025.md).

No server action, service/clock change, product load, dependency update or
qualification occurs. The qualification manifest remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
