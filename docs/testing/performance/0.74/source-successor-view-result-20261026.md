# Exact source successor validation for historical artifact tests

The thirteen historical body and COMPAT failures located after the inventory
repair are closed in `diagnostic_artifact_fixture_074`: all twenty unchanged
cases pass on Rust 1.94 and MSRV 1.88. Five new successor controls and fourteen
inventory controls also pass on both compilers. This is a focused source-preservation result,
not a full package, runtime safety or release admission result.

Implementation commit: `67807a6aaec8b006277df04b2e0eac797ddba102`.

## Five independently described inputs

The [separate source successor](../../../../tools/source-successor-view-074/README.md)
checks complete current canonical SHA256 values, applies exact inverse edits at
fixed byte offsets, then checks complete predecessor SHA256 values. Its expected
description is reconstructed from fixed producer commits, not the current index,
HEAD, a failed assertion's output or an observed response.

| Input | Current producer | Predecessor | Inverse edits |
| --- | --- | --- | --- |
| COMPAT | `a67a5299ab335424e9b11330dc06bd749ea9b859` | `3c52c0aa34b8d00b2fac44375d5ba8fdf532bb9f` | 1 |
| Gated test registry | `958b3135a15c00b9f6aad41522cef9d1f6608d3a` | `3c52c0aa34b8d00b2fac44375d5ba8fdf532bb9f` | 1 |
| Authority fixed | `958b3135a15c00b9f6aad41522cef9d1f6608d3a` | `c91068bfa8b297bb075b63457da235aaa9c6232e` | 3 |
| Authority fixture | same | same | 16 |
| Kernel origin | same | same | 17 |

The authority changes replace fresh metadata materialization with the previously
measured fresh-fstat helpers. The kernel delta includes fresh identities and
test-only attribution/rejected namespace and filesystem experiments. The registry
adds thirteen manual controls. COMPAT records the diagnostic TLS material,
allocation-v2 format and terminal GET-owner rollback. Mapping these deltas for an
old test does not accept any rejected performance candidate or independently
prove runtime equivalence.

The complete description has SHA256
`61d3e1941cdae497600592a49af0487e4cf545573ba3f091add173110b560cda`,
pinned independently in the Rust reader. `generate.py` checks its reconstruction
with replacement objects disabled. Its write mode creates exclusively and cannot
refresh a file in place. LF checkout attributes preserve the compiled data pin.

## Preserved checks and fresh observations

The original IO reader, owned-case contract, inventory ledgers, five original
inventory tests and all twenty artifact cases remain byte-identical in canonical
LF form. All 437/438/440/441/442/443/446 comparison counts remain unchanged.
The existing owned-case layer already accounts for the launcher test's three-line
addition; adding a second exemption would be unnecessary and incorrect.

Only the two historical entry points acquire a projection before their existing
checks. Their primitive reviewed-byte predicates and pins remain unchanged; those
old-phase predicates are not standalone latest-source approval APIs. Unknown
paths pass through without normalization. Known paths reject stale predecessors,
partial installs, missing/duplicate edits, wrong-file bytes and unrelated drift
before exposing the old phase. No new filesystem or Git read occurs in this
projection, and only immutable expected rules are cached.

The new controls independently compare both ends to fixed Git producer bodies,
exercise every inverse edit's missing and duplicate forms, reject cross-path
substitutions, verify exact old gate bodies apart from their named entry edits,
and reject a changed observation after a success. The full old artifact tests
then exercise their original comparisons and mutation assertions. This is more
than changing expected output, but less than a proof of arbitrary semantics or
atomic filesystem observations.

## First outcomes and evidence

The new canary first fails at the current COMPAT input, with unchanged source and
HEAD. The first implementation then fails compilation with E0277: the selected
SHA array does not implement `LowerHex`. Replacing that formatting operation with
the repository's byte-wise hexadecimal conversion fixes compilation without a
dependency change. Both raw refusals and their input hashes remain separate from
the subsequent green candidate.

The first compiling candidate passes twenty original artifact cases and five new
controls, with no ignored or filtered cases. MSRV also passes all twenty artifact
cases and the five new plus nine current-phase/five original inventory cases.
All-target xtask check, strict all-target/all-feature Clippy, scoped xtask
formatting, doc-check and local non-promotable performance-contract check pass.
The final current-compiler cohort repeats the five new plus fourteen inventory
cases without changing source. Workspace-wide formatting is not claimed: its
previous Windows command-line-length refusal remains separate.

The [step packet](local-runs/source-successor-view-20261026/README.md) retains
79 members in 66 raw objects, with index SHA256
`fc8a4d25330d0a4b4701ba072ac55bebee67d9f40e9450b0b9a301913cf1c215`.
Its offline verifier and six mutation controls pass. The replay checks both
producer bodies, all 38 inverse edits, exact cohort accounting and preservation
of the first refusals without executing retained code or rerunning tests.

The first wider [package attempt](package-closure-v4-result-20261025.md), its
780-second guard, interrupted target, unstarted suffix and raw failures stay
unchanged. The earlier [7 passed and 13 failed artifact result](inventory544-phase-result-20261026.md)
also remains intact. No server action, live preparation, service or clock change,
dependency update, product measurement or qualification is performed here.

## Next package boundary

Continue the authority/runtime/current-tree and bootstrap compatibility frontiers
from the retained failed-case inventory. Do not assume this one green target
closes them, and do not infer panic details for the interrupted target from its
case names. Preserve their old assertions and first results. Review the final
changed source/effect graph independently before another whole-package attempt;
the old v4 review must keep refusing changed source rather than acquiring new
observed pins. Scope, dependency/MSRV policy, manager support, original execution,
all-writer retirement, crash/reconnect and real enrollment remain the six-gate
closure status, not consequences of this source compatibility repair.
