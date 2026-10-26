# Authority reader phase validation

Five remaining failures in the selected authority/current-tree cohort are closed.
All 22 old cases, six new routing/preservation cases and five unchanged successor
controls pass on Rust 1.94 and MSRV 1.88. This repairs source-test compatibility;
bootstrap checks, the complete current-tree audit and the full package are not
certified by this subset.

Implementation commit: `4e7989f3a86321907234ec6116e1c4aa023d00b1`.

## Distinguish real refusals from expected mutation panics

The retained original full-package stdout shows expected caught context-hook
panics, then the actual uncaught failure at the changed gated-test registry.
The short audit excerpt alone could not establish that distinction. The new
read-only seven-target baseline confirms context, key-hex and current-tree-reader
targets are already green after the previous source-successor repair: 17 cases
pass and five fail across the other four targets. No hook predicate needs removal.

That baseline is on exact development HEAD
`628bc3c113ce12e9ecb10ee4daee6961a6b2ad15`, not a retry of the unchanged original
full package. Its complete logs and source/HEAD receipts remain in the
[step packet](local-runs/authority-reader-phase-20261026/README.md), alongside
the derived original context section and its stdout hash/offset/length.

## Separate current entry inputs from historical body inputs

The registry reader tests now verify the latest complete registry using the
existing source successor before exercising their original old-byte predicates.
The historical-input test preserves both the original entry and its verified
predecessor: the old primitive receives the latter, while the current entry point
receives the former. Substituting the old value at both places would make the
current gate reject a stale input for the right reason.

The authority-runtime test makes the same distinction for the fresh-fstat fixture.
Historical candidate/body assertions receive its verified predecessor. The
current `view` receives freshly observed actual fixture bytes. Negative current
mutants and the stale predecessor still refuse; the original primitive's semantics
and the actual gate are unchanged. Each declared read still performs one physical
source read before its relevant projection. These are test-source observations,
not new live host or process admission checks.

Five files change: `authority_hex_reader_074.rs`, `owned_case_074.rs`,
`historical_input_view_074.rs`, `authority_runtime_074.rs` and
`current_tree_reader_074.rs`. The
[exact construction contract](../../../../tools/authority-reader-phase-074/README.md)
pins their complete prior hashes and describes only the named input edits.
Independent fixed Git sources reproduce every current file. AST controls retain
all eighteen cases in those five files and every original assertion after
reversing two explicitly named input-expression substitutions. Four cases in the
untouched context/key-hex targets remain independent controls. No comparison,
expected predicate, mutation loop or case is removed.

## Account for the terminal GET-owner rollback without reviving it

The first candidate closes three failures and reaches a further exact boundary:
the main performance-contract source changed when GET-owner was terminally
retired. Its 27-case cohort has 25 passes and two failures. That first result is
kept rather than retried without a change.

A separate one-file description maps the terminal source at
`a67a5299ab335424e9b11330dc06bd749ea9b859` to
`37db65f68e67ed7c0ee0f1cc84fed1c30bf9822a`. It verifies the whole current source,
reverses five exact text edits and verifies the whole predecessor before the old
authority assertions. The rules are independently pinned at SHA256
`9373e36370e3893316a983fb159debba29afbbf4095c86f8f71d4384c80cc4f5`;
the fixed-commit generator checks their reconstruction and cannot overwrite them.
Partial/duplicate edits, stale predecessors and unrelated body drift refuse.

Only a string is returned to the old source assertions. It is never compiled,
executed or written into the current main file. The actual performance-contract
source, product runtime and feature tombstone remain unchanged. This is neither
an acceptance of GET-owner nor evidence of a new runtime or numerical benefit.

## Focused results and remaining boundary

The final current-compiler candidate passes 28 cases: 22 original and six new.
A separate unchanged-successor cohort adds five controls on the same compiled
sources. MSRV 1.88 runs all 33 in one invocation. There are no ignored or filtered
cases, changed local bounds or deadline extensions. Raw before/after snapshots
bind source and HEAD throughout each invocation.

All-target xtask check, all-target/all-feature strict Clippy, scoped xtask
formatting, doc-check and local non-promotable performance-contract check pass.
The step packet has 74 members in 63 raw objects, with index SHA256
`8022ce3470effdd0f4c1df62747cf076f2c42c5420d6b3146b08c70534a03417`.
Offline replay and six controls pass, including rejection of erased first
failures, a replaced old reader and a substituted terminal producer after
rehashing the evidence index.
Workspace-wide formatting and a complete package pass are not claimed.

The original full-package failure, earlier artifact failures and both new first
refusals remain immutable. No dependency update, server action, service or clock
change, live preparation, product measurement or qualification is performed.
The actual runtime/IO/owned-case/source-successor gates and qualification inputs
are unchanged. This step does not establish arbitrary semantic equivalence or an
atomic filesystem observation.

Next, repair the remaining bootstrap compatibility views and complete current-tree
audit. The independently reviewed source/effect graph must cover the final tree
before a new whole-package attempt within the original 780-second/four-thread
guard. Do not infer the interrupted target's thirty panic causes from their case
names or combine this subset with its old incomplete result. Security policy,
release scope, manager support and production execution/enrollment remain separate
release gates.
