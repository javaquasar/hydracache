# Windows package effect review and first v2 replay

A separate manual effect scope now covers the actual default Windows source.
Its first full-command replay is red: seven source-compatibility cases fail,
and the original 780-second guard interrupts `performance_contract_074` again.
Effect clearance is not semantic compatibility, package completion or release
admission. No package or performance cohort is retried.

Subsequent [focused compatibility repair](package-compatibility-isolation-result-20261025.md)
restores four old sources and closes the seven cases on current/MSRV compilers.
It leaves this first replay red and the v2 admission pins unchanged; v2 refuses
the repaired current tree until a new source review.

## Separate reviewed scope

Code commit `baffe6000f747ea6bc830f338f2673b0874129b7` adds
`package_effects_review_v2.py`, `package_replay_v2.py`, their controls and the
retained syntax fixture. The old reviewer and its literal pins stay unchanged;
an actual invocation still refuses the current graph before dispatch.

The first bounded syntax capture closes 342 Rust sources and 25129 sites with no
unresolved module edge: eight added sources, four changed old sources and no
removed old source. Canonical AST SHA256 is
`befe75505aa2c8c6da32424c23792063bcb096bf1352a42ee8a255e16ccf0840`.
The [syntax packet](local-runs/package-effect-syntax-v2-20261025/index.json)
retains both source-only rounds, build/source receipts and staged verification.
Its index SHA256 is
`54795bdd931fa4b759621d5fae7ec76d430e1611d82a139c7200252d4bc6d447`.

Manual delta review covers default-off helper observations, pure event/contract
code, opt-in normalization runners, the exact path predicate/gate adapter and
the new self-child pipe target. The latter runs six same-executable fixture
children and two expected missing-name spawn failures, not its opt-in Git
measurement cohort. New inert include-data and upstream reviewer/parser inputs
expand the auxiliary ledger from 21 to 28 fixed hashes. This remains a bounded
manual AST/entry-point review, not arbitrary macro expansion, a sandbox theorem,
Linux/all-feature clearance or production process-tree retirement.

V2 preserves the old fixture/Git/scratch checks and adds refusals for inherited
build overrides, external or shadowing Cargo configuration and an unexpectedly
present negative-control executable. Only the original wrapper's exact
`CARGO_INCREMENTAL=0` and `CARGO_BUILD_JOBS=2` controls are allowed. This machine
passes without removing variables or changing configuration. Unsupported future
configuration must receive review, not automatic sanitization.

## Coordinator and focused controls

The new coordinator performs preflight by default. Only explicit `--execute`
dispatches the fixed unfiltered command, four case threads and the original
780-second wrapper. Both wrapper byte hashes must match. Source inputs must be
clean, including untracked files. An exclusive destination and intent precede
preflight; failures remain and no automatic retry is implemented. The coordinator
does not infer a package pass from dispatch or a successful preflight.
These closed diagnostic files are not a new fsync/crash-durability protocol.

Eighty-one historical-tool Python controls and fifty-three attribution controls
pass. Eighteen new controls cover exact scope/delta/auxiliary pins, stale legacy
refusal, host constructors/includes, source/module drift, build/config overrides,
missing-program presence, default non-dispatch, fixed argv, dirty-source refusal,
early failure records, wrapper drift and exclusive destinations.
All-target check, strict all-target/all-feature Clippy and Python syntax/indentation
pass. No Rust behavior, dependencies, original live predicates or 300/60/10 windows
change in this step.

The first expanded control run has two fixture errors: clearing env removes HOME,
and a mocked path comparison overlooks Windows short-path normalization. Its
source/logs remain. Corrected fixtures retain the production checks. The first
v2 preflight before adding build/config checks is kept separately from the final
stronger preflight; neither is relabeled as a package result.

## First complete command attempt

The sole attempt uses the clean code commit above and exactly:

```text
cargo +1.94.0 test --offline --locked --no-fail-fast -p xtask --lib --tests -- --test-threads=4
```

| First outcome | Count |
| --- | ---: |
| Catalogued targets | 136 |
| Completed targets | 112 |
| Passing cases in completed targets | 815 |
| Failed cases in completed targets | 7 |
| Passing cases observed in incomplete target | 171 of 321 declared |
| Unstarted targets | 23 |

The 986 individual passing observations are not a complete passing package.
The incomplete target has no completion verdict or reported target duration.
Build and completed-target spans are not comparable product performance, and
their difference from the nominal budget is not attributed overhead. New cases,
short-circuit failures and concurrency prevent a speedup inference against the
older replay. Source and HEAD stay unchanged; outer result is `outer-timeout`.

### Source compatibility failures

Source inspection joins the actual errors to these expected-view mismatches;
it does not prove a repair or authorize refreshing expected bytes from a result.

| Failed target | Located mismatch |
| --- | --- |
| `artifact_inventory_reader_074` | Old complete-helper expectation lacks additive observers. |
| `authority_runtime_074` | Old one-reader expectation lacks the accepted exact path-predicate edit. |
| `authority_rust_history_074` | Current-tree input refuses changed `authority_context_074`. |
| `current_tree_input_audit_074` | Two unaccounted views: authority context and artifact helper. |
| `current_tree_reader_074` | Declared nine-patch expectation rejects authority context. |
| `historical_source_cache_074` | Old complete gate expectation lacks the path-predicate adapter. |
| `owned_case_074` | Old complete authority-context expectation lacks diagnostic additions. |

The artifact's twenty-case target and new pipe target themselves pass. That does
not waive these seven independent failures. Next isolate diagnostic additions
from frozen-source readers where possible; where a test-only projection is
necessary, require exact independent construction/erasure and missing/duplicate/
assertion-drift controls. Do not add path exclusions, broaden the old nine-patch
ledger silently, accept observed output as its own pin or suppress a failed case.

Only after focused repairs and a new manually reviewed changed-source scope may
another full replay be considered, with the same command/780-second guard.
Compatibility repairs alone do not establish that the remaining 321-case target
can finish in the window. Its cost remains a separate unresolved owner.

## First outcome retention and safety boundaries

The cleanup log contains one failed PID termination message. A later read-only
query finds none of the five queried PIDs, but original creation identities were
not recorded for them. This is current absence only, not proof about earlier
lifetimes, escaped descendants, descriptor aliases or original all-writer custody.
No additional process is stopped by that query.

The first offline renderer saves the valid strict report, then raises KeyError
while formatting a missing `failed` field on the incomplete target. Its source is
retained. A separate reader reconciles that same saved report against the same
raw logs without another producer; its note is explicitly a transcription of the
tool result, not a captured process log.

The [review and replay packet](local-runs/package-effect-review-v2-20261025/index.json)
retains first controls/refusals, final preflight, wrappers, unchanged-source maps,
raw package/cleanup logs, strict derived outcomes and staged verification.
Its index SHA256 is
`58b7a5f845abd8e791b5d150f98f461fc069869195131fe3e76ea620df772dae`.
The old package timeout, all seven new failures and unfinished suffix remain red.
Fifteen dependency advisories remain audit-only; C74, original delivery, writer
cessation, completion/reconnect and production enrollment remain unresolved.
No frozen 0.73 artifact, service, clock, server, product load or expensive
qualification is changed or run.
